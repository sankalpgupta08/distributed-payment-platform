use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct IdempotencyRecord {
    pub request_hash: String,
    pub response_body: Option<Value>,
    pub status_code: Option<i16>,
}

#[derive(sqlx::FromRow)]
struct IdempotencyRow {
    request_hash: String,
    response_body: Option<Value>,
    status_code: Option<i16>,
}

impl From<IdempotencyRow> for IdempotencyRecord {
    fn from(row: IdempotencyRow) -> Self {
        Self {
            request_hash: row.request_hash,
            response_body: row.response_body,
            status_code: row.status_code,
        }
    }
}

/// Removes an expired record before a key is considered for reuse.
pub async fn delete_expired(
    transaction: &mut Transaction<'_, Postgres>,
    key: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM idempotency_keys WHERE key = $1 AND expires_at <= NOW()")
        .bind(key)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}

/// Attempts to claim a previously unused idempotency key.
pub async fn try_insert(
    transaction: &mut Transaction<'_, Postgres>,
    id: Uuid,
    key: &str,
    request_hash: &str,
    expires_at: DateTime<Utc>,
) -> Result<bool, sqlx::Error> {
    let inserted = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO idempotency_keys (id, key, request_hash, expires_at)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (key) DO NOTHING
        RETURNING id
        "#,
    )
    .bind(id)
    .bind(key)
    .bind(request_hash)
    .bind(expires_at)
    .fetch_optional(&mut **transaction)
    .await?;

    Ok(inserted.is_some())
}

/// Loads an idempotency record while preventing concurrent mutation.
pub async fn find_by_key_for_update(
    transaction: &mut Transaction<'_, Postgres>,
    key: &str,
) -> Result<Option<IdempotencyRecord>, sqlx::Error> {
    let row = sqlx::query_as::<_, IdempotencyRow>(
        r#"
        SELECT request_hash, response_body, status_code
        FROM idempotency_keys
        WHERE key = $1
        FOR UPDATE
        "#,
    )
    .bind(key)
    .fetch_optional(&mut **transaction)
    .await?;

    Ok(row.map(Into::into))
}

/// Reads a non-expired idempotency record without acquiring a database lock.
pub async fn find_active_by_key(
    pool: &PgPool,
    key: &str,
) -> Result<Option<IdempotencyRecord>, sqlx::Error> {
    let row = sqlx::query_as::<_, IdempotencyRow>(
        r#"
        SELECT request_hash, response_body, status_code
        FROM idempotency_keys
        WHERE key = $1 AND expires_at > NOW()
        "#,
    )
    .bind(key)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(Into::into))
}

/// Stores the exact HTTP response to replay for later duplicate requests.
pub async fn store_response(
    transaction: &mut Transaction<'_, Postgres>,
    key: &str,
    response_body: &Value,
    status_code: i16,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE idempotency_keys
        SET response_body = $2, status_code = $3
        WHERE key = $1
        "#,
    )
    .bind(key)
    .bind(response_body)
    .bind(status_code)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}
