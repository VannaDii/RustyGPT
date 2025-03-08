use axum::serve;
use tokio::net::TcpListener;

mod handlers;
mod routes;

#[tokio::main]
async fn main() {
    let app = routes::create_router();
    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
    serve(listener, app).await.unwrap();
}
