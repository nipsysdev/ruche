use crate::models::BeeData;
use crate::services::db_service::BeeDatabase;
use crate::AppState;
use anyhow::{anyhow, Error};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Serialize;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

#[derive(Debug, Serialize)]
pub struct CustomError {
    pub message: String,
}

impl From<Error> for CustomError {
    fn from(err: Error) -> Self {
        CustomError {
            message: err.to_string(),
        }
    }
}

impl IntoResponse for CustomError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(self)).into_response()
    }
}

pub fn init_bees_handler(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", post(create_bee))
        .route("/", get(get_bees))
        .route("/{bee_id}", delete(delete_bee))
        .route("/{bee_id}/req", delete(request_bee_deletion))
        .with_state(app_state)
}

async fn create_bee(State(state): State<Arc<AppState>>) -> Result<Json<Vec<BeeData>>, CustomError> {
    state
        .bee_service
        .save_bee()
        .await
        .map(|bee| vec![bee])
        .map(Json)
        .map_err(Into::into)
}

async fn get_bees(State(state): State<Arc<AppState>>) -> Result<Json<Vec<BeeData>>, CustomError> {
    state
        .bee_service
        .get_bees()
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn request_bee_deletion(
    Path(bee_id): Path<u8>,
    State(state): State<Arc<AppState>>,
) -> Result<(), CustomError> {
    let mut last_bee_deletion_req = state.last_bee_deletion_req.lock().await;
    last_bee_deletion_req.insert(bee_id, SystemTime::now());
    Ok(())
}

async fn delete_bee(
    Path(bee_id): Path<u8>,
    State(state): State<Arc<AppState>>,
) -> Result<(), CustomError> {
    let mut last_bee_deletion_req = state.last_bee_deletion_req.lock().await;

    let has_made_request = match last_bee_deletion_req.get(&bee_id) {
        Some(last_deletion_req) => match last_deletion_req.elapsed() {
            Ok(duration) => duration < Duration::from_secs(30),
            Err(_) => false,
        },
        None => false,
    };

    if (!has_made_request) {
        return Err(anyhow!(
            "Unable to confirm deletion of bee with id {}. No request made in last 30sec.",
            bee_id
        )
        .into());
    }

    state.bee_service.delete_bee(bee_id).await?;

    last_bee_deletion_req.remove(&bee_id);

    Ok(())
}
