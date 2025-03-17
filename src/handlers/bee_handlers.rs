use crate::error::HttpError;
use crate::models::bee::BeeData;
use crate::services::bee_service::BeeService;
use crate::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, post};
use axum::{Json, Router};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

pub fn init_bee_handlers(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", post(create_bee))
        .route("/{bee_id}", delete(delete_bee))
        .route("/{bee_id}/req", delete(request_bee_deletion))
        .with_state(app_state)
}

async fn create_bee(State(state): State<Arc<AppState>>) -> Result<Json<BeeData>, HttpError> {
    if !state.bee_service.ensure_capacity().await? {
        return Err(HttpError::new(
            StatusCode::BAD_REQUEST,
            &format!(
                "Max capacity reached. {} bee nodes already registered.",
                state.bee_service.count_bees().await?
            ),
        ));
    }

    let new_bee_id = state.bee_service.get_new_bee_id().await?;

    let neighborhood = BeeService::get_neighborhood().await?;

    state.bee_service.create_node_dir(new_bee_id).await?;

    let bee_data = BeeData {
        id: new_bee_id,
        neighborhood,
        reserve_doubling: true,
    };

    state
        .bee_service
        .save_bee(bee_data)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn request_bee_deletion(
    Path(bee_id): Path<u8>,
    State(state): State<Arc<AppState>>,
) -> Result<(), HttpError> {
    ensure_bee_exists(bee_id, state.clone()).await?;

    let mut last_bee_deletion_req = state.last_bee_deletion_req.lock().await;
    last_bee_deletion_req.insert(bee_id, SystemTime::now());
    Ok(())
}

async fn delete_bee(
    Path(bee_id): Path<u8>,
    State(state): State<Arc<AppState>>,
) -> Result<(), HttpError> {
    ensure_bee_exists(bee_id, state.clone()).await?;

    let mut last_bee_deletion_req = state.last_bee_deletion_req.lock().await;

    let has_made_request = match last_bee_deletion_req.get(&bee_id) {
        Some(last_deletion_req) => match last_deletion_req.elapsed() {
            Ok(duration) => duration < Duration::from_secs(30),
            Err(_) => false,
        },
        None => false,
    };

    if !has_made_request {
        return Err(HttpError::new(
            StatusCode::BAD_REQUEST,
            &format!(
                "Unable to confirm deletion of bee with id {}. No request made in last 30sec.",
                bee_id
            ),
        ));
    }

    state.bee_service.delete_bee(bee_id).await?;

    last_bee_deletion_req.remove(&bee_id);

    Ok(())
}

async fn ensure_bee_exists(bee_id: u8, state: Arc<AppState>) -> Result<(), HttpError> {
    match state.bee_service.get_bee(bee_id).await? {
        Some(_) => Ok(()),
        None => Err(HttpError::new(
            StatusCode::NOT_FOUND,
            &format!("Unable to find bee with id {}.", bee_id),
        )),
    }
}
