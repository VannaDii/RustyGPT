use std::sync::Arc;

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
};
use serde::Serialize;

use crate::{app_state::AppState, db::bootstrap};

#[derive(Serialize)]
struct HealthResponse<'a> {
    status: &'a str,
}

#[derive(Serialize)]
struct ReadyzResponse {
    ready: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    missing: Vec<&'static str>,
}

async fn healthz() -> impl IntoResponse {
    metrics::counter!("health_checks_total", "endpoint" => "healthz", "status" => "ok")
        .increment(1);
    (StatusCode::OK, Json(HealthResponse { status: "ok" }))
}

async fn readyz(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    if let Some(pool) = state.pool.as_ref() {
        match bootstrap::readiness_state(pool).await {
            Ok(status) if status.ready => {
                metrics::counter!(
                    "health_checks_total",
                    "endpoint" => "readyz",
                    "status" => "ok"
                )
                .increment(1);
                (
                    StatusCode::OK,
                    Json(ReadyzResponse {
                        ready: true,
                        missing: Vec::new(),
                    }),
                )
            }
            Ok(status) => {
                metrics::counter!(
                    "health_checks_total",
                    "endpoint" => "readyz",
                    "status" => "incomplete"
                )
                .increment(1);
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(ReadyzResponse {
                        ready: false,
                        missing: status.missing.clone(),
                    }),
                )
            }
            Err(_) => {
                metrics::counter!(
                    "health_checks_total",
                    "endpoint" => "readyz",
                    "status" => "error"
                )
                .increment(1);
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(ReadyzResponse {
                        ready: false,
                        missing: vec!["readiness_check_failed"],
                    }),
                )
            }
        }
    } else {
        metrics::counter!(
            "health_checks_total",
            "endpoint" => "readyz",
            "status" => "error"
        )
        .increment(1);
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ReadyzResponse {
                ready: false,
                missing: vec!["database_connection_unavailable"],
            }),
        )
    }
}

pub fn create_health_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use serde_json::Value;
    use serial_test::serial;
    use sqlx::postgres::PgPoolOptions;
    use std::io;
    use tower::ServiceExt;

    #[tokio::test]
    async fn healthz_returns_ok() {
        let _ = crate::server::metrics_handle();
        let app = create_health_router().with_state(Arc::new(AppState::default()));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    fn test_pool() -> sqlx::PgPool {
        PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://postgres:postgres@localhost:5432/rustygpt_test")
            .expect("lazy pool creation should succeed")
    }

    #[tokio::test]
    #[serial]
    async fn readyz_returns_ready_when_database_is_healthy() {
        let _ = crate::server::metrics_handle();
        crate::db::bootstrap::set_readiness_override(Some(Ok(
            crate::db::bootstrap::BootstrapStatus {
                ready: true,
                missing: vec![],
            },
        )));

        let state = Arc::new(AppState {
            pool: Some(test_pool()),
            assistant: None,
            sse_store: None,
            sessions: None,
            rate_limits: None,
            streams: None,
        });

        let app = create_health_router().with_state(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/readyz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ready"], serde_json::json!(true));
        crate::db::bootstrap::set_readiness_override(None);
    }

    #[tokio::test]
    #[serial]
    async fn readyz_returns_service_unavailable_when_database_fails() {
        let _ = crate::server::metrics_handle();
        crate::db::bootstrap::set_readiness_override(Some(Err(sqlx::Error::Io(io::Error::other(
            "simulated failure",
        )))));

        let state = Arc::new(AppState {
            pool: Some(test_pool()),
            assistant: None,
            sse_store: None,
            sessions: None,
            rate_limits: None,
            streams: None,
        });

        let app = create_health_router().with_state(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/readyz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        crate::db::bootstrap::set_readiness_override(None);
    }
}
