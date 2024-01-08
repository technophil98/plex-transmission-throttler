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
use crate::transmission::new_transmission_client;

mod app;
mod plex;
mod transmission;

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

    let state = AppState::new(new_transmission_client()?);
    let app = app()?.with_state(Arc::new(state));

    axum::serve(listener, app).await.unwrap();

    Ok(())
}

pub(crate) fn app() -> anyhow::Result<Router<Arc<AppState>>> {
    let app = Router::new()
        .route("/", get(health_check))
        .route("/", post(webhook))
        .layer(TraceLayer::new_for_http());

    Ok(app)
}

async fn health_check() -> impl IntoResponse {
    Json(json!({"status": "ok"}))
}

#[derive(Debug, Deserialize)]
struct WebhookPayload {
    action: Action,
    stream_location: StreamLocation,
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

            transmission_client.enable_transmission_alt_speed().await?;

            tracing::info!("Enabled alt speed");

            Ok(Json(json!({ "status": "Throttling enabled" })))
        }
        Action::Pause | Action::Stop => {
            tracing::debug!("Disabling alt speed");

            transmission_client.disable_transmission_alt_speed().await?;

            tracing::info!("Disabled alt speed");

            Ok(Json(json!({ "status": "Throttling disabled" })))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::transmission::MockTransmissionClient;
    use anyhow::bail;
    use assertables::{assert_contains, assert_contains_as_result};
    use axum::response::Response;
    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use rstest::*;
    use serde_json::{json, Value};
    use tokio::sync::Mutex;
    use tower::ServiceExt;

    use super::*;

    #[fixture]
    fn app() -> Router {
        let mock_state = Arc::new(AppState {
            transmission_client: Arc::new(Mutex::new(MockTransmissionClient)),
        });

        super::app()
            .expect("App should init properly")
            .with_state(mock_state)
    }

    #[rstest]
    #[tokio::test]
    async fn health_check(app: Router) -> anyhow::Result<()> {
        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await?;

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await?.to_bytes();
        let body: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(body, json!({"status": "ok"}));

        Ok(())
    }

    async fn send_webhook_request(
        app: Router,
        request_body: &Value,
    ) -> Result<Response, http::Error> {
        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(serde_json::to_vec(&request_body).unwrap()))?,
            )
            .await?;

        Ok(response)
    }

    #[rstest]
    #[case(json!({"action": "play", "stream_location": "wan"}), json!({"status": "Throttling enabled"}))]
    #[case(json!({"action": "pause", "stream_location": "wan"}), json!({"status": "Throttling disabled"}))]
    #[case(json!({"action": "resume", "stream_location": "wan"}), json!({"status": "Throttling enabled"}))]
    #[case(json!({"action": "stop", "stream_location": "wan"}), json!({"status": "Throttling disabled"}))]
    #[case(json!({"action": "play", "stream_location": "lan"}), json!({"status": "Stream location 'Lan' does not require throttling."}))]
    #[tokio::test]
    async fn test_webhook_200(
        app: Router,
        #[case] request_body: Value,
        #[case] expected_response: Value,
    ) -> anyhow::Result<()> {
        let response = send_webhook_request(app, &request_body).await?;

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await?.to_bytes();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body, expected_response);

        Ok(())
    }

    #[rstest]
    #[case(json!({"action": "play"}), "Failed to deserialize the JSON body into the target type: missing field `stream_location`")]
    #[case(json!({"stream_location": "wan"}), "Failed to deserialize the JSON body into the target type: missing field `action`")]
    #[case(json!({"action": "unknown", "stream_location": "wan"}), "Failed to deserialize the JSON body into the target type: action: unknown variant `unknown`, expected one of `play`, `pause`, `resume`, `stop`")]
    #[tokio::test]
    async fn test_webhook_invalid_payload(
        app: Router,
        #[case] request_body: Value,
        #[case] expected_message: String,
    ) -> anyhow::Result<()> {
        let response = send_webhook_request(app, &request_body).await?;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let body = response.into_body().collect().await?.to_bytes();
        let body: Value = serde_json::from_slice(&body).unwrap();

        if let Some(Value::String(message)) = body.get("error") {
            assert_contains!(message, &expected_message);
        } else {
            bail!("Response doesn't have 'error' field");
        }

        Ok(())
    }
}
