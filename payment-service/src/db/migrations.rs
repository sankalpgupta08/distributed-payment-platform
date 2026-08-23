use sqlx::{
    PgPool,
    migrate::{MigrateError, Migrator},
};

/// SQL migration files embedded in the application at compile time.
pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

/// Applies every migration that has not yet been recorded by SQLx.
pub async fn run(pool: &PgPool) -> Result<(), MigrateError> {
    MIGRATOR.run(pool).await
}
