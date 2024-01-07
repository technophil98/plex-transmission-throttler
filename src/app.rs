use std::sync::Arc;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use tokio::sync::Mutex;
use transmission_rpc::TransClient;

pub(crate) struct AppState {
    pub transmission_client: Arc<Mutex<TransClient>>,
}

impl AppState {
    pub fn new(transmission_client: TransClient) -> Self {
        Self {
            transmission_client: Arc::new(Mutex::new(transmission_client)),
        }
    }
}

pub(crate) struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Something went wrong: {}", self.0)})),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
