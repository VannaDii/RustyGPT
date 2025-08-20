use axum::{body::Body, http::Request};
use std::time::Duration;
use tower_http::classify::{ServerErrorsAsFailures, ServerErrorsFailureClass, SharedClassifier};
use tower_http::trace::{
    DefaultMakeSpan, DefaultOnBodyChunk, DefaultOnEos, DefaultOnResponse, TraceLayer,
};
use tracing::{Level, Span, error, info};

// Add this type alias for the complex type.
type TraceLayerType = TraceLayer<
    SharedClassifier<ServerErrorsAsFailures>,
    DefaultMakeSpan,
    fn(&Request<Body>, &Span) -> (),
    DefaultOnResponse,
    DefaultOnBodyChunk,
    DefaultOnEos,
    fn(ServerErrorsFailureClass, Duration, &Span) -> (),
>;

/// Handle incoming request logging
pub(crate) fn on_request_handler(req: &Request<Body>, span: &Span) {
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
pub(crate) fn on_failure_handler(error: ServerErrorsFailureClass, latency: Duration, span: &Span) {
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
        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
        .on_request(on_request_handler as fn(&Request<Body>, &Span))
        .on_response(DefaultOnResponse::new().level(Level::INFO))
        .on_failure(on_failure_handler as fn(ServerErrorsFailureClass, Duration, &Span))
}
