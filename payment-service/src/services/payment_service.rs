use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::{
        payment::Payment,
        requests::{CreatePaymentRequest, UpdatePaymentStatusRequest},
    },
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

/// Moves a payment through one allowed lifecycle transition.
pub async fn update_payment_status(
    pool: &PgPool,
    payment_id: Uuid,
    request: UpdatePaymentStatusRequest,
) -> Result<Payment, AppError> {
    let current_payment = get_payment(pool, payment_id).await?;
    let requested_status = request.status;

    if !current_payment.status.can_transition_to(requested_status) {
        return Err(AppError::conflict(format!(
            "cannot transition payment from {} to {}",
            current_payment.status, requested_status
        )));
    }

    payment_repository::update_status(pool, payment_id, current_payment.status, requested_status)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| {
            AppError::conflict(
                "payment state changed concurrently; retrieve the payment and retry if appropriate",
            )
        })
}
