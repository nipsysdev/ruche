use crate::error::HttpError;
use crate::models::BeeData;
use crate::AppState;
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use std::sync::Arc;

pub fn init_bees_handlers(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(get_bees))
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
