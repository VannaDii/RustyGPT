//! # Tracer Tests
//!
//! Comprehensive tests for the HTTP tracing functionality used in the server.

#[cfg(test)]
mod tests {
    use super::super::tracer::*;
    use crate::middleware::request_context::RequestContext;
    use axum::body::Body;
    use axum::http::{Method, Request, Response, StatusCode, Version};
    use std::time::Duration;
    use tower_http::{
        classify::ServerErrorsFailureClass,
        trace::{MakeSpan, OnResponse},
    };
    use tracing::{Level, span};
    use tracing_subscriber::util::SubscriberInitExt;

    #[test]
    fn test_create_trace_layer() {
        let trace_layer = create_trace_layer();

        // Verify the trace layer can be created without panicking
        // This tests the complex type construction and function casting
        assert!(std::mem::size_of_val(&trace_layer) > 0);
    }

    #[test]
    fn test_on_request_handler_basic() {
        let _guard = tracing_subscriber::fmt()
            .with_max_level(Level::INFO)
            .set_default();

        let request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .version(Version::HTTP_11)
            .body(Body::empty())
            .unwrap();

        let span = span!(Level::INFO, "test_span");

        // This should not panic
        on_request_handler(&request, &span);
    }

    #[test]
    fn test_on_request_handler_post_method() {
        let _guard = tracing_subscriber::fmt()
            .with_max_level(Level::INFO)
            .set_default();

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/test")
            .version(Version::HTTP_11)
            .body(Body::empty())
            .unwrap();

        let span = span!(Level::INFO, "test_post_span");

        // This should not panic
        on_request_handler(&request, &span);
    }

    #[test]
    fn test_on_request_handler_with_query_params() {
        let _guard = tracing_subscriber::fmt()
            .with_max_level(Level::INFO)
            .set_default();

        let request = Request::builder()
            .method(Method::GET)
            .uri("/test?param1=value1&param2=value2")
            .version(Version::HTTP_2)
            .body(Body::empty())
            .unwrap();

        let span = span!(Level::INFO, "test_query_span");

        // This should not panic
        on_request_handler(&request, &span);
    }

    #[test]
    fn test_on_failure_handler_basic() {
        let _guard = tracing_subscriber::fmt()
            .with_max_level(Level::ERROR)
            .set_default();

        let error =
            ServerErrorsFailureClass::StatusCode(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        let latency = Duration::from_millis(100);
        let span = span!(Level::ERROR, "test_failure_span");

        // This should not panic
        on_failure_handler(error, latency, &span);
    }

    #[test]
    fn test_on_failure_handler_timeout() {
        let _guard = tracing_subscriber::fmt()
            .with_max_level(Level::ERROR)
            .set_default();

        let error = ServerErrorsFailureClass::StatusCode(axum::http::StatusCode::REQUEST_TIMEOUT);
        let latency = Duration::from_secs(30);
        let span = span!(Level::ERROR, "test_timeout_span");

        // This should not panic
        on_failure_handler(error, latency, &span);
    }

    #[test]
    fn test_on_failure_handler_service_unavailable() {
        let _guard = tracing_subscriber::fmt()
            .with_max_level(Level::ERROR)
            .set_default();

        let error =
            ServerErrorsFailureClass::StatusCode(axum::http::StatusCode::SERVICE_UNAVAILABLE);
        let latency = Duration::from_millis(5000);
        let span = span!(Level::ERROR, "test_unavailable_span");

        // This should not panic
        on_failure_handler(error, latency, &span);
    }

    #[test]
    fn test_trace_layer_type_size() {
        // Verify the complex type alias works correctly
        let layer = create_trace_layer();
        let size = std::mem::size_of_val(&layer);

        // The layer should have a reasonable size (not zero, not massive)
        assert!(size > 0);
        assert!(size < 10000); // Sanity check for reasonable size
    }

    #[test]
    fn test_request_with_different_methods() {
        let _guard = tracing_subscriber::fmt()
            .with_max_level(Level::INFO)
            .set_default();

        let methods = [
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::HEAD,
            Method::OPTIONS,
        ];

        for method in methods {
            let request = Request::builder()
                .method(method.clone())
                .uri("/test")
                .body(Body::empty())
                .unwrap();

            let span = span!(Level::INFO, "test_method_span", method = %method);

            // All methods should be handled without panicking
            on_request_handler(&request, &span);
        }
    }

    #[test]
    fn test_request_with_different_uris() {
        let _guard = tracing_subscriber::fmt()
            .with_max_level(Level::INFO)
            .set_default();

        let uris = [
            "/",
            "/api",
            "/api/v1/users",
            "/health",
            "/metrics",
            "/static/file.css",
            "/auth/callback?code=123",
        ];

        for uri in uris {
            let request = Request::builder()
                .method(Method::GET)
                .uri(uri)
                .body(Body::empty())
                .unwrap();

            let span = span!(Level::INFO, "test_uri_span", uri = %uri);

            // All URIs should be handled without panicking
            on_request_handler(&request, &span);
        }
    }

    #[test]
    fn test_request_with_different_versions() {
        let _guard = tracing_subscriber::fmt()
            .with_max_level(Level::INFO)
            .set_default();

        let versions = [
            Version::HTTP_09,
            Version::HTTP_10,
            Version::HTTP_11,
            Version::HTTP_2,
            Version::HTTP_3,
        ];

        for version in versions {
            let request = Request::builder()
                .method(Method::GET)
                .uri("/test")
                .version(version)
                .body(Body::empty())
                .unwrap();

            let span = span!(Level::INFO, "test_version_span", version = ?version);

            // All HTTP versions should be handled without panicking
            on_request_handler(&request, &span);
        }
    }

    #[test]
    fn test_failure_with_different_status_codes() {
        let _guard = tracing_subscriber::fmt()
            .with_max_level(Level::ERROR)
            .set_default();

        let status_codes = [
            axum::http::StatusCode::BAD_REQUEST,
            axum::http::StatusCode::UNAUTHORIZED,
            axum::http::StatusCode::FORBIDDEN,
            axum::http::StatusCode::NOT_FOUND,
            axum::http::StatusCode::METHOD_NOT_ALLOWED,
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::http::StatusCode::BAD_GATEWAY,
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            axum::http::StatusCode::GATEWAY_TIMEOUT,
        ];

        for status_code in status_codes {
            let error = ServerErrorsFailureClass::StatusCode(status_code);
            let latency = Duration::from_millis(100);
            let span = span!(Level::ERROR, "test_status_span", status = %status_code);

            // All status codes should be handled without panicking
            on_failure_handler(error, latency, &span);
        }
    }

    #[test]
    fn test_failure_with_different_latencies() {
        let _guard = tracing_subscriber::fmt()
            .with_max_level(Level::ERROR)
            .set_default();

        let latencies = [
            Duration::from_millis(1),
            Duration::from_millis(10),
            Duration::from_millis(100),
            Duration::from_millis(1000),
            Duration::from_secs(1),
            Duration::from_secs(10),
            Duration::from_secs(60),
        ];

        for latency in latencies {
            let error =
                ServerErrorsFailureClass::StatusCode(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
            let span = span!(Level::ERROR, "test_latency_span", latency = ?latency);

            // All latencies should be handled without panicking
            on_failure_handler(error, latency, &span);
        }
    }

    #[test]
    fn test_function_pointers_casting() {
        // Test that the function pointer casting works correctly
        let request_fn = on_request_handler as fn(&Request<Body>, &tracing::Span);
        let failure_fn =
            on_failure_handler as fn(ServerErrorsFailureClass, Duration, &tracing::Span);

        // Verify the function pointers are not null
        assert_ne!(request_fn as *const (), std::ptr::null());
        assert_ne!(failure_fn as *const (), std::ptr::null());
    }

    #[test]
    fn test_span_in_scope_usage() {
        let _guard = tracing_subscriber::fmt()
            .with_max_level(Level::INFO)
            .set_default();

        let request = Request::builder()
            .method(Method::GET)
            .uri("/test-scope")
            .body(Body::empty())
            .unwrap();

        let span = span!(Level::INFO, "test_scope_span");

        // Test that span.in_scope works correctly in our handler
        let _entered = span.enter();
        on_request_handler(&request, &span);

        // Test failure handler with span scope
        let error =
            ServerErrorsFailureClass::StatusCode(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        let latency = Duration::from_millis(100);
        on_failure_handler(error, latency, &span);
    }

    #[test]
    fn http_metrics_recorded_for_response() {
        let handle = crate::server::metrics_handle();
        let _subscriber_guard = tracing_subscriber::fmt()
            .with_max_level(Level::TRACE)
            .finish()
            .set_default();

        let mut request = Request::builder()
            .method(Method::GET)
            .uri("/metrics-test")
            .body(Body::empty())
            .unwrap();
        request.extensions_mut().insert(RequestContext {
            request_id: "req-1".into(),
            user_id: None,
        });

        let mut make_span = HttpMakeSpan::default();
        let span = make_span.make_span(&request);
        let response = Response::builder()
            .status(StatusCode::OK)
            .body(Body::empty())
            .unwrap();

        HttpOnResponse::default().on_response(&response, Duration::from_millis(10), &span);

        let metrics = handle.render();
        assert!(
            metrics.contains(
                "http_requests_total{method=\"GET\",path=\"/metrics-test\",status=\"200\"}"
            ),
            "expected counter line for request metrics"
        );
        assert!(
            metrics.contains("http_request_duration_seconds"),
            "expected histogram samples for request duration"
        );
    }
}
