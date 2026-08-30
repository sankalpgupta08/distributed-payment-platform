//! Structured logging initialization for the service process.

use tracing_subscriber::{EnvFilter, fmt};

pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("payment_service=info,tower_http=info"));

    fmt()
        .json()
        .with_current_span(true)
        .with_span_list(true)
        .with_env_filter(filter)
        .init();
}
