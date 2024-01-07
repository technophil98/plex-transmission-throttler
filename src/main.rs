use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_extra::extract::WithRejection;
use serde::Deserialize;
use serde_json::{json, Value};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::app::{AppError, AppState};
use crate::plex::{Action, StreamLocation, UNTHROTTLED_STREAM_LOCATIONS};
use crate::transmission::{new_transmission_client, set_transmission_alt_speed};

mod app;
mod plex;
mod transmission;

#[derive(Debug, Deserialize)]
struct WebhookPayload {
    action: Action,
    stream_location: StreamLocation,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "plex_transmission_throttler=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();

    tracing::info!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app()?).await.unwrap();

    Ok(())
}

pub(crate) fn app() -> anyhow::Result<Router> {
    let state = AppState::new(new_transmission_client()?);

    let app = Router::new()
        .route("/", get(health_check))
        .route("/", post(webhook))
        .layer(TraceLayer::new_for_http())
        .with_state(Arc::new(state));

    Ok(app)
}

async fn health_check() -> impl IntoResponse {
    Json(json!({"status": "ok"}))
}

async fn webhook(
    State(state): State<Arc<AppState>>,
    WithRejection(Json(payload), _): WithRejection<Json<WebhookPayload>, AppError>,
) -> Result<Json<Value>, AppError> {
    let action = payload.action;
    let stream_location = payload.stream_location;

    if UNTHROTTLED_STREAM_LOCATIONS.contains(&stream_location) {
        let message = format!("Stream location '{stream_location}' does not require throttling.");
        tracing::info!(message);
        return Ok(Json(json!({"status": message})));
    }

    tracing::info!("Received action '{action}' for stream location '{stream_location}'");

    let mut transmission_client = state.transmission_client.lock().await;

    match action {
        Action::Play | Action::Resume => {
            tracing::debug!("Enabling alt speed");

            set_transmission_alt_speed(&mut transmission_client, true).await?;

            tracing::info!("Enabled alt speed");

            Ok(Json(json!({ "status": "Throttling enabled" })))
        }
        Action::Pause | Action::Stop => {
            tracing::debug!("Disabling alt speed");

            set_transmission_alt_speed(&mut transmission_client, false).await?;

            tracing::info!("Disabled alt speed");

            Ok(Json(json!({ "status": "Throttling disabled" })))
        }
    }
}
