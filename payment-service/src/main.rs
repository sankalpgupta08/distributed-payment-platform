use axum::{
    routing::get,
    Json,
    Router,
};
use serde::Serialize;
use tokio::net::TcpListener;

#[derive(Serialize)]
struct RootResponse {
    service: String,
    version: String,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
}

async fn root() -> Json<RootResponse> {
    Json(RootResponse {
        service: "payment-service".to_string(),
        version: "0.1.0".to_string(),
    })
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
    })
}

#[tokio::main]
async fn main() {
    // Create all application routes
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health));

    // Bind server to localhost:3000
    let listener = TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("Failed to bind to address");

    println!("==================================");
    println!(" Payment Service Started");
    println!(" Listening on http://127.0.0.1:3000");
    println!("==================================");

    // Start accepting requests
    axum::serve(listener, app)
        .await
        .expect("Server failed");
}