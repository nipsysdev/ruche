use crate::models::bee::{BeeData, BeeInfo};
use crate::models::http_error::HttpError;
use crate::AppState;
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use std::sync::Arc;

pub fn init_bees_handlers(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(get_bees))
        .route("/recreate", get(recreate_bees))
        .with_state(app_state)
}

async fn get_bees(State(state): State<Arc<AppState>>) -> Result<Json<Vec<BeeData>>, HttpError> {
    state
        .bee_service
        .get_bees()
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn recreate_bees(State(state): State<Arc<AppState>>) -> Result<(), HttpError> {
    let bees = state
        .bee_service
        .get_bees()
        .await?
        .into_iter()
        .map(|bd| state.bee_service.bee_data_to_info(&bd))
        .collect::<anyhow::Result<Vec<_>>>()?;

    state
        .bee_service
        .recreate_bee_containers(bees)
        .await
        .map_err(Into::into)
}
