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

async fn healthz() -> impl IntoResponse {
    metrics::counter!("health_checks_total", "endpoint" => "healthz", "status" => "ok")
        .increment(1);
    (StatusCode::OK, Json(HealthResponse { status: "ok" }))
}

async fn readyz(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    if let Some(pool) = state.pool.as_ref() {
        match bootstrap::ensure_readiness(pool).await {
            Ok(_) => {
                metrics::counter!(
                    "health_checks_total",
                    "endpoint" => "readyz",
                    "status" => "ok"
                )
                .increment(1);
                (StatusCode::OK, Json(HealthResponse { status: "ready" }))
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
                    Json(HealthResponse { status: "degraded" }),
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
            Json(HealthResponse { status: "no_db" }),
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
    use axum::{body::Body, http::Request};
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
    async fn readyz_returns_ready_when_database_is_healthy() {
        let _ = crate::server::metrics_handle();
        crate::db::bootstrap::set_readiness_override(Some(Ok(())));

        let state = Arc::new(AppState {
            pool: Some(test_pool()),
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
        crate::db::bootstrap::set_readiness_override(None);
    }

    #[tokio::test]
    async fn readyz_returns_service_unavailable_when_database_fails() {
        let _ = crate::server::metrics_handle();
        crate::db::bootstrap::set_readiness_override(Some(Err(sqlx::Error::Io(io::Error::new(
            io::ErrorKind::Other,
            "simulated failure",
        )))));

        let state = Arc::new(AppState {
            pool: Some(test_pool()),
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
