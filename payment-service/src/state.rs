use sqlx::PgPool;

use crate::config::AppConfig;

/// Dependencies shared by HTTP handlers.
///
/// The PostgreSQL pool is added in Phase 3. A Redis client will be added in
/// Phase 9.
#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub db: PgPool,
}

impl AppState {
    pub fn new(config: AppConfig, db: PgPool) -> Self {
        Self { config, db }
    }
}
