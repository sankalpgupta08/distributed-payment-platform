use axum::{
    Router,
    http::StatusCode,
    routing::{get, patch, post},
};
use tower_http::{limit::RequestBodyLimitLayer, timeout::TimeoutLayer, trace::TraceLayer};

use crate::{
    handlers::{
        health::{health, ready},
        payments::{create_payment, get_payment, update_payment_status},
        root::root,
    },
    state::AppState,
};

/// Composes every HTTP endpoint exposed by this service.
pub fn create_router(state: AppState) -> Router {
    let request_timeout = state.config.request_timeout;
    let max_request_body_bytes = state.config.max_request_body_bytes;

    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/payments", post(create_payment))
        .route("/payments/{id}", get(get_payment))
        .route("/payments/{id}/status", patch(update_payment_status))
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            request_timeout,
        ))
        .layer(RequestBodyLimitLayer::new(max_request_body_bytes))
        .with_state(state)
}
