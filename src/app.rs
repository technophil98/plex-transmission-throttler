use std::sync::Arc;

use crate::transmission::TransmissionClient;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use thiserror::Error;
use tokio::sync::Mutex;
use transmission_rpc::TransClient;

pub(crate) struct AppState {
    pub transmission_client: Arc<Mutex<dyn TransmissionClient>>,
}

impl AppState {
    pub fn new(transmission_client: TransClient) -> Self {
        Self {
            transmission_client: Arc::new(Mutex::new(transmission_client)),
        }
    }
}

#[derive(Debug, Error)]
pub(crate) enum AppError {
    #[error(transparent)]
    JsonExtractorRejection(#[from] JsonRejection),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::JsonExtractorRejection(json_rejection) => {
                (json_rejection.status(), json_rejection.body_text())
            }
            AppError::Other(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Something went wrong: {}", e),
            ),
        };

        let json_payload = json!({"error": message });
        (status, Json(json_payload)).into_response()
    }
}
