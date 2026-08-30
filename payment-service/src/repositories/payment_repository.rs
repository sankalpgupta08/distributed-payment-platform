use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::payment::{NewPayment, Payment, PaymentStatus};

#[derive(sqlx::FromRow)]
struct PaymentRow {
    id: Uuid,
    merchant_id: Uuid,
    amount: Decimal,
    currency: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<PaymentRow> for Payment {
    type Error = sqlx::Error;

    fn try_from(row: PaymentRow) -> Result<Self, Self::Error> {
        let status = PaymentStatus::try_from(row.status.as_str())
            .map_err(|error| sqlx::Error::Decode(Box::new(error)))?;

        Ok(Self {
            id: row.id,
            merchant_id: row.merchant_id,
            amount: row.amount,
            currency: row.currency,
            status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

/// Inserts one payment and returns PostgreSQL's stored representation.
pub async fn create(pool: &PgPool, payment: NewPayment) -> Result<Payment, sqlx::Error> {
    let row = sqlx::query_as::<_, PaymentRow>(
        r#"
        INSERT INTO payments (id, merchant_id, amount, currency, status)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, merchant_id, amount, currency, status, created_at, updated_at
        "#,
    )
    .bind(payment.id)
    .bind(payment.merchant_id)
    .bind(payment.amount)
    .bind(payment.currency)
    .bind(payment.status.as_str())
    .fetch_one(pool)
    .await?;

    row.try_into()
}

/// Finds one payment by its stable public identifier.
pub async fn find_by_id(pool: &PgPool, payment_id: Uuid) -> Result<Option<Payment>, sqlx::Error> {
    let row = sqlx::query_as::<_, PaymentRow>(
        r#"
        SELECT id, merchant_id, amount, currency, status, created_at, updated_at
        FROM payments
        WHERE id = $1
        "#,
    )
    .bind(payment_id)
    .fetch_optional(pool)
    .await?;

    row.map(TryInto::try_into).transpose()
}

/// Updates a payment only when its database state still matches `expected_status`.
///
/// The expected-state condition prevents a concurrent request from overwriting
/// a transition that completed after the service initially read the payment.
pub async fn update_status(
    pool: &PgPool,
    payment_id: Uuid,
    expected_status: PaymentStatus,
    next_status: PaymentStatus,
) -> Result<Option<Payment>, sqlx::Error> {
    let row = sqlx::query_as::<_, PaymentRow>(
        r#"
        UPDATE payments
        SET status = $3
        WHERE id = $1 AND status = $2
        RETURNING id, merchant_id, amount, currency, status, created_at, updated_at
        "#,
    )
    .bind(payment_id)
    .bind(expected_status.as_str())
    .bind(next_status.as_str())
    .fetch_optional(pool)
    .await?;

    row.map(TryInto::try_into).transpose()
}
