use axum::{
    Router,
    routing::{get, patch, post},
};

use crate::{
    handlers::{
        health::health,
        payments::{create_payment, get_payment, update_payment_status},
        root::root,
    },
    state::AppState,
};

/// Composes every HTTP endpoint exposed by this service.
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/payments", post(create_payment))
        .route("/payments/{id}", get(get_payment))
        .route("/payments/{id}/status", patch(update_payment_status))
        .with_state(state)
}
