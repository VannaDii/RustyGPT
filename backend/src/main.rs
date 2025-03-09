use axum::{
    Router,
    extract::Extension,
    http::{Method, StatusCode},
    middleware::{self, Next},
    response::Response,
    serve,
};
use http::Request;
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::net::TcpListener;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
};

mod handlers;
mod routes;
mod services;

#[cfg(test)]
mod main_test;

// Middleware to check if a user is authenticated
pub async fn auth_middleware(
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // In a real application, you would check for a valid JWT token or session
    // For now, we'll just pass through all requests
    Ok(next.run(req).await)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize environment variables (in a real app, use dotenv or similar)
    println!("Starting server...");

    // Set up database connection pool
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/rusty_gpt".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create database connection pool");

    // Set up CORS
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any);

    // Create the auth router
    let auth_router = routes::auth::create_router_auth();

    // Create the protected router with middleware
    let protected_router = routes::protected::create_router_protected()
        .route_layer(middleware::from_fn(auth_middleware));

    // Set up static file serving for the auth success page
    let frontend_path = PathBuf::from("../frontend");

    // Combine routers
    let app = Router::new()
        .merge(auth_router)
        .merge(protected_router)
        .nest_service("/", ServeDir::new(frontend_path))
        .layer(cors)
        .layer(Extension(pool));

    // Start the server
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener = TcpListener::bind(addr).await?;
    println!("Listening on {}", addr);

    serve(listener, app).await?;

    Ok(())
}
