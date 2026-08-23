use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::{payment::Payment, requests::CreatePaymentRequest},
    repositories::payment_repository,
};

/// Validates a create request and persists a payment in its initial state.
pub async fn create_payment(
    pool: &PgPool,
    request: CreatePaymentRequest,
) -> Result<Payment, AppError> {
    let new_payment = request.into_new_payment(Uuid::new_v4())?;
    payment_repository::create(pool, new_payment)
        .await
        .map_err(AppError::from)
}

/// Retrieves a payment or reports that its identifier does not exist.
pub async fn get_payment(pool: &PgPool, payment_id: Uuid) -> Result<Payment, AppError> {
    payment_repository::find_by_id(pool, payment_id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found(format!("payment {payment_id} was not found")))
}
