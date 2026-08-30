use std::time::Duration;

use sqlx::{PgPool, postgres::PgPoolOptions};

const MIN_DATABASE_CONNECTIONS: u32 = 1;
const CONNECTION_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);

/// Creates a reusable PostgreSQL connection pool and verifies connectivity.
///
/// `connect` opens an initial connection, so startup fails immediately if the
/// configured database is unavailable or the connection string is invalid.
pub async fn create_pool(database_url: &str, max_connections: u32) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .min_connections(MIN_DATABASE_CONNECTIONS)
        .acquire_timeout(CONNECTION_ACQUIRE_TIMEOUT)
        .connect(database_url)
        .await
}
