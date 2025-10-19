use axum::{
    body::Body,
    extract::MatchedPath,
    http::{Request, Response},
};
use std::time::Duration;
use tower_http::classify::{ServerErrorsAsFailures, ServerErrorsFailureClass, SharedClassifier};
use tower_http::trace::{DefaultOnBodyChunk, DefaultOnEos, MakeSpan, OnResponse, TraceLayer};
use tracing::{Span, error, info};
use tracing_subscriber::registry::{LookupSpan, Registry};

use crate::middleware::request_context::RequestContext;

// Add this type alias for the complex type.
type TraceLayerType = TraceLayer<
    SharedClassifier<ServerErrorsAsFailures>,
    HttpMakeSpan,
    fn(&Request<Body>, &Span) -> (),
    HttpOnResponse,
    DefaultOnBodyChunk,
    DefaultOnEos,
    fn(ServerErrorsFailureClass, Duration, &Span) -> (),
>;

#[derive(Clone, Default)]
struct RequestMetadata {
    method: String,
    path: String,
}

#[derive(Clone, Default)]
pub struct HttpMakeSpan;

impl<B> MakeSpan<B> for HttpMakeSpan {
    fn make_span(&mut self, request: &Request<B>) -> Span {
        let request_id = request
            .extensions()
            .get::<RequestContext>()
            .map(|ctx| ctx.request_id.clone())
            .unwrap_or_else(|| "n/a".into());

        let method = request.method().to_string();
        let path = request
            .extensions()
            .get::<MatchedPath>()
            .map(|p| p.as_str().to_string())
            .unwrap_or_else(|| request.uri().path().to_string());

        let span = tracing::info_span!(
            "http_request",
            method = %request.method(),
            uri = %request.uri(),
            matched_path = %path,
            request_id = %request_id,
            status_code = tracing::field::Empty,
            latency_seconds = tracing::field::Empty
        );
        span.with_subscriber(|(id, dispatch)| {
            if let Some(registry) = dispatch.downcast_ref::<Registry>() {
                if let Some(span_ref) = registry.span(id) {
                    span_ref
                        .extensions_mut()
                        .insert(RequestMetadata { method, path });
                }
            }
        });
        span
    }
}

#[derive(Clone, Default)]
pub struct HttpOnResponse;

impl<B> OnResponse<B> for HttpOnResponse {
    fn on_response(self, response: &Response<B>, latency: Duration, span: &Span) {
        let status = response.status();
        span.record("status_code", status.as_u16());

        let latency_seconds = latency.as_secs_f64();
        span.record("latency_seconds", latency_seconds);

        span.in_scope(|| {
            if status.is_server_error() {
                tracing::error!(
                    latency_seconds,
                    status = status.as_u16(),
                    "finished processing request"
                );
            } else {
                tracing::info!(
                    latency_seconds,
                    status = status.as_u16(),
                    "finished processing request"
                );
            }
        });

        let mut meta = None;
        span.with_subscriber(|(id, dispatch)| {
            if let Some(registry) = dispatch.downcast_ref::<Registry>() {
                if let Some(span_ref) = registry.span(id) {
                    if let Some(stored) = span_ref.extensions().get::<RequestMetadata>() {
                        meta = Some(stored.clone());
                    }
                }
            }
        });

        if let Some(meta) = meta {
            let RequestMetadata { method, path } = meta;
            let status_label = status.as_u16().to_string();
            metrics::counter!(
                "http_requests_total",
                "method" => method.clone(),
                "path" => path.clone(),
                "status" => status_label.clone()
            )
            .increment(1);
            metrics::histogram!(
                "http_request_duration_seconds",
                "method" => method,
                "path" => path,
                "status" => status_label
            )
            .record(latency_seconds);
        }
    }
}

/// Handle incoming request logging
pub fn on_request_handler(req: &Request<Body>, span: &Span) {
    span.in_scope(|| {
        info!(
            method = %req.method(),
            uri = %req.uri(),
            version = ?req.version(),
            "started processing request"
        );
    })
}

/// Handle failure logging
pub fn on_failure_handler(error: ServerErrorsFailureClass, latency: Duration, span: &Span) {
    span.in_scope(|| {
        error!(
            error = %error,
            latency = ?latency,
            "error processing request"
        );
    })
}

/// Create a trace layer for HTTP request logging
pub fn create_trace_layer() -> TraceLayerType {
    TraceLayer::new_for_http()
        .make_span_with(HttpMakeSpan::default())
        .on_request(on_request_handler as fn(&Request<Body>, &Span))
        .on_response(HttpOnResponse::default())
        .on_failure(on_failure_handler as fn(ServerErrorsFailureClass, Duration, &Span))
}
