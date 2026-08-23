use axum::{Json, extract::State};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct RootResponse {
    pub service: &'static str,
    pub version: &'static str,
}

/// Basic service metadata endpoint.
pub async fn root(State(_state): State<AppState>) -> Json<RootResponse> {
    Json(RootResponse {
        service: "payment-service",
        version: env!("CARGO_PKG_VERSION"),
    })
}
