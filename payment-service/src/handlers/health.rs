use axum::{Json, extract::State};
use serde::Serialize;

use crate::{errors::AppError, state::AppState};

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

/// Liveness endpoint. It confirms that the HTTP process is running.
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "healthy" })
}

/// Readiness endpoint. It confirms that required dependencies are reachable.
pub async fn ready(State(state): State<AppState>) -> Result<Json<HealthResponse>, AppError> {
    sqlx::query("SELECT 1").execute(&state.db).await?;
    state.redis_locks.ping().await?;

    Ok(Json(HealthResponse { status: "ready" }))
}
