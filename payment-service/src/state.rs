use sqlx::PgPool;

use crate::{config::AppConfig, locks::RedisLock};

/// Dependencies shared by HTTP handlers.
///
/// PostgreSQL remains the durable source of truth; Redis is used for short
/// lived idempotency coordination.
#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub db: PgPool,
    pub redis_locks: RedisLock,
}

impl AppState {
    pub fn new(config: AppConfig, db: PgPool, redis_locks: RedisLock) -> Self {
        Self {
            config,
            db,
            redis_locks,
        }
    }
}
