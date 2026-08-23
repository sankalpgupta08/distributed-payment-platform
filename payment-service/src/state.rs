use crate::config::AppConfig;

/// Dependencies shared by HTTP handlers.
///
/// The database pool and Redis client will be added here in Phases 3 and 9.
#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }
}
