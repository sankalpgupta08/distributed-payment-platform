use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

/// Liveness endpoint. It confirms that the HTTP process is running.
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "healthy" })
}
