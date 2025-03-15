use crate::models::BeeData;
use crate::services::db_service::BeeDatabase;
use crate::AppState;
use anyhow::Error;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Serialize;
use std::sync::Arc;

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
