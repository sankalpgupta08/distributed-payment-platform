use axum::{
    Router,
    routing::{get, post},
};

use crate::{
    handlers::{
        health::health,
        payments::{create_payment, get_payment},
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
        .with_state(state)
}
