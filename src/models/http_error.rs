use anyhow::Error;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct HttpError {
    #[serde(skip_serializing)]
    status_code: StatusCode,
    message: String,
}

impl HttpError {
    pub fn new(status_code: StatusCode, message: &str) -> Self {
        HttpError {
            status_code,
            message: message.to_string(),
        }
    }
}

impl From<Error> for HttpError {
    fn from(err: Error) -> Self {
        HttpError {
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
            message: err.to_string(),
        }
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        (self.status_code, Json(self)).into_response()
    }
}
