use axum::{extract::Request, middleware::Next, response::Response};
use http::StatusCode;
use tracing::{info, instrument};

// Middleware to check if a user is authenticated
#[instrument(skip(next))]
pub async fn auth_middleware(
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // In a real application, you would check for a valid JWT token or session
    // For now, we'll just pass through all requests
    info!(
        "Auth middleware processing request to: {}",
        req.uri().path()
    );
    Ok(next.run(req).await)
}
