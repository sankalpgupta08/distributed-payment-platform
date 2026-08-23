use axum::{Router, routing::get};

use crate::{
    handlers::{health::health, root::root},
    state::AppState,
};

/// Composes every HTTP endpoint exposed by this service.
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .with_state(state)
}
