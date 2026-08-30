use std::time::Duration;

use chrono::Utc;
use sqlx::PgPool;
use tracing::warn;
use uuid::Uuid;

use crate::{
    errors::AppError,
    locks::RedisLock,
    models::{
        payment::{NewPayment, Payment},
        requests::{CreatePaymentRequest, UpdatePaymentStatusRequest},
    },
    repositories::{idempotency_repository, payment_repository},
};

const MAX_SERIALIZATION_ATTEMPTS: usize = 3;
const CREATED_STATUS_CODE: i16 = 201;
const IDEMPOTENCY_REPLAY_WAIT_ATTEMPTS: usize = 20;
const IDEMPOTENCY_REPLAY_WAIT: Duration = Duration::from_millis(25);

enum TransitionError {
    PaymentNotFound,
    InvalidTransition {
        current: crate::models::payment::PaymentStatus,
        requested: crate::models::payment::PaymentStatus,
    },
    ConcurrentStateChange,
    Database(sqlx::Error),
}

pub struct PaymentCreationResult {
    pub payment: Payment,
    pub status_code: u16,
}

enum IdempotentCreationError {
    Conflict(String),
    Database(sqlx::Error),
    InvalidStoredResponse,
}

/// Creates a payment exactly once from the API client's perspective.
///
/// Redis reduces duplicate-request contention, while PostgreSQL's unique key
/// and transaction remain the durable correctness mechanism.
pub async fn create_payment(
    pool: &PgPool,
    redis_locks: &RedisLock,
    idempotency_ttl: Duration,
    idempotency_key: &str,
    request: CreatePaymentRequest,
) -> Result<PaymentCreationResult, AppError> {
    validate_idempotency_key(idempotency_key)?;

    let request_hash = request.request_hash();
    let new_payment = request.into_new_payment(Uuid::new_v4())?;
    let lock_key = format!("payment-service:idempotency:{idempotency_key}");

    match redis_locks.acquire(lock_key).await {
        Ok(Some(lock_guard)) => {
            let result = create_payment_durably(
                pool,
                idempotency_ttl,
                idempotency_key,
                &request_hash,
                new_payment,
            )
            .await;

            if let Err(error) = lock_guard.release().await {
                warn!(error = %error, "failed to release Redis idempotency lock");
            }

            result
        }
        Ok(None) => {
            replay_existing_or_report_in_progress(pool, idempotency_key, &request_hash).await
        }
        Err(error) => {
            // PostgreSQL's unique key is still sufficient for correctness, so
            // Redis becoming temporarily unavailable must not create duplicates.
            warn!(error = %error, "Redis unavailable; falling back to PostgreSQL idempotency");
            create_payment_durably(
                pool,
                idempotency_ttl,
                idempotency_key,
                &request_hash,
                new_payment,
            )
            .await
        }
    }
}

async fn create_payment_durably(
    pool: &PgPool,
    idempotency_ttl: Duration,
    idempotency_key: &str,
    request_hash: &str,
    new_payment: NewPayment,
) -> Result<PaymentCreationResult, AppError> {
    for attempt in 1..=MAX_SERIALIZATION_ATTEMPTS {
        match create_payment_once(
            pool,
            idempotency_ttl,
            idempotency_key,
            request_hash,
            new_payment.clone(),
        )
        .await
        {
            Ok(result) => return Ok(result),
            Err(IdempotentCreationError::Conflict(message)) => {
                return Err(AppError::conflict(message));
            }
            Err(IdempotentCreationError::Database(error))
                if is_serialization_failure(&error) && attempt < MAX_SERIALIZATION_ATTEMPTS =>
            {
                tokio::task::yield_now().await;
            }
            Err(IdempotentCreationError::Database(error)) if is_serialization_failure(&error) => {
                return Err(AppError::conflict(
                    "payment creation conflicted repeatedly; retry with the same idempotency key",
                ));
            }
            Err(IdempotentCreationError::Database(error)) => return Err(AppError::from(error)),
            Err(IdempotentCreationError::InvalidStoredResponse) => {
                warn!(
                    idempotency_key,
                    "stored idempotency response could not be decoded"
                );
                return Err(AppError::Internal);
            }
        }
    }

    unreachable!("the loop returns after its final creation attempt")
}

async fn create_payment_once(
    pool: &PgPool,
    idempotency_ttl: Duration,
    idempotency_key: &str,
    request_hash: &str,
    new_payment: NewPayment,
) -> Result<PaymentCreationResult, IdempotentCreationError> {
    let mut transaction = pool
        .begin()
        .await
        .map_err(IdempotentCreationError::Database)?;

    sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut *transaction)
        .await
        .map_err(IdempotentCreationError::Database)?;

    idempotency_repository::delete_expired(&mut transaction, idempotency_key)
        .await
        .map_err(IdempotentCreationError::Database)?;

    let claimed_key = idempotency_repository::try_insert(
        &mut transaction,
        Uuid::new_v4(),
        idempotency_key,
        request_hash,
        Utc::now()
            + chrono::Duration::from_std(idempotency_ttl)
                .expect("configured idempotency TTL must fit in chrono duration"),
    )
    .await
    .map_err(IdempotentCreationError::Database)?;

    if !claimed_key {
        let record =
            idempotency_repository::find_by_key_for_update(&mut transaction, idempotency_key)
                .await
                .map_err(IdempotentCreationError::Database)?
                .ok_or_else(|| {
                    IdempotentCreationError::Conflict(
                        "idempotency key was removed while being processed; retry the request"
                            .to_owned(),
                    )
                })?;

        if record.request_hash != request_hash {
            return Err(IdempotentCreationError::Conflict(
                "idempotency key was already used with a different request".to_owned(),
            ));
        }

        let result = record_to_creation_result(record)?;
        transaction
            .commit()
            .await
            .map_err(IdempotentCreationError::Database)?;
        return Ok(result);
    }

    let payment = payment_repository::create_in_transaction(&mut transaction, new_payment)
        .await
        .map_err(IdempotentCreationError::Database)?;
    let response_body = serde_json::to_value(&payment)
        .map_err(|_| IdempotentCreationError::InvalidStoredResponse)?;

    idempotency_repository::store_response(
        &mut transaction,
        idempotency_key,
        &response_body,
        CREATED_STATUS_CODE,
    )
    .await
    .map_err(IdempotentCreationError::Database)?;

    transaction
        .commit()
        .await
        .map_err(IdempotentCreationError::Database)?;

    Ok(PaymentCreationResult {
        payment,
        status_code: CREATED_STATUS_CODE as u16,
    })
}

async fn replay_existing_or_report_in_progress(
    pool: &PgPool,
    idempotency_key: &str,
    request_hash: &str,
) -> Result<PaymentCreationResult, AppError> {
    for _ in 0..IDEMPOTENCY_REPLAY_WAIT_ATTEMPTS {
        if let Some(record) = idempotency_repository::find_active_by_key(pool, idempotency_key)
            .await
            .map_err(AppError::from)?
        {
            if record.request_hash != request_hash {
                return Err(AppError::conflict(
                    "idempotency key was already used with a different request",
                ));
            }

            if record.response_body.is_some() {
                return record_to_creation_result(record).map_err(|error| match error {
                    IdempotentCreationError::Conflict(message) => AppError::conflict(message),
                    IdempotentCreationError::InvalidStoredResponse => AppError::Internal,
                    IdempotentCreationError::Database(error) => AppError::from(error),
                });
            }
        }

        tokio::time::sleep(IDEMPOTENCY_REPLAY_WAIT).await;
    }

    Err(AppError::conflict(
        "an identical payment request is still being processed; retry with the same idempotency key",
    ))
}

fn record_to_creation_result(
    record: idempotency_repository::IdempotencyRecord,
) -> Result<PaymentCreationResult, IdempotentCreationError> {
    let response_body = record.response_body.ok_or_else(|| {
        IdempotentCreationError::Conflict(
            "an identical payment request is currently being processed".to_owned(),
        )
    })?;
    let status_code = record
        .status_code
        .ok_or(IdempotentCreationError::InvalidStoredResponse)?;
    let payment = serde_json::from_value(response_body)
        .map_err(|_| IdempotentCreationError::InvalidStoredResponse)?;

    Ok(PaymentCreationResult {
        payment,
        status_code: status_code as u16,
    })
}

fn validate_idempotency_key(key: &str) -> Result<(), AppError> {
    if key.is_empty() || key.len() > 255 {
        return Err(AppError::bad_request(
            "Idempotency-Key must contain between 1 and 255 characters",
        ));
    }

    Ok(())
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
    for attempt in 1..=MAX_SERIALIZATION_ATTEMPTS {
        match transition_status_once(pool, payment_id, request.status).await {
            Ok(payment) => return Ok(payment),
            Err(TransitionError::PaymentNotFound) => {
                return Err(AppError::not_found(format!(
                    "payment {payment_id} was not found"
                )));
            }
            Err(TransitionError::InvalidTransition { current, requested }) => {
                return Err(AppError::conflict(format!(
                    "cannot transition payment from {current} to {requested}"
                )));
            }
            Err(TransitionError::ConcurrentStateChange) => {
                return Err(AppError::conflict(
                    "payment state changed concurrently; retrieve the payment and retry if appropriate",
                ));
            }
            Err(TransitionError::Database(error))
                if is_serialization_failure(&error) && attempt < MAX_SERIALIZATION_ATTEMPTS =>
            {
                // PostgreSQL aborted this transaction to preserve serializable
                // behavior. Retrying starts with a fresh snapshot.
                tokio::task::yield_now().await;
            }
            Err(TransitionError::Database(error)) if is_serialization_failure(&error) => {
                return Err(AppError::conflict(
                    "payment update conflicted repeatedly; retrieve the payment and retry",
                ));
            }
            Err(TransitionError::Database(error)) => return Err(AppError::from(error)),
        }
    }

    unreachable!("the loop returns after its final transition attempt")
}

async fn transition_status_once(
    pool: &PgPool,
    payment_id: Uuid,
    requested_status: crate::models::payment::PaymentStatus,
) -> Result<Payment, TransitionError> {
    let mut transaction = pool.begin().await.map_err(TransitionError::Database)?;

    // This must be the transaction's first database statement.
    sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut *transaction)
        .await
        .map_err(TransitionError::Database)?;

    let current_payment = payment_repository::find_by_id_for_update(&mut transaction, payment_id)
        .await
        .map_err(TransitionError::Database)?
        .ok_or(TransitionError::PaymentNotFound)?;

    if !current_payment.status.can_transition_to(requested_status) {
        return Err(TransitionError::InvalidTransition {
            current: current_payment.status,
            requested: requested_status,
        });
    }

    let updated_payment = payment_repository::update_status(
        &mut transaction,
        payment_id,
        current_payment.status,
        requested_status,
    )
    .await
    .map_err(TransitionError::Database)?
    .ok_or(TransitionError::ConcurrentStateChange)?;

    transaction
        .commit()
        .await
        .map_err(TransitionError::Database)?;

    Ok(updated_payment)
}

fn is_serialization_failure(error: &sqlx::Error) -> bool {
    matches!(
        error,
        sqlx::Error::Database(database_error)
            if database_error.code().as_deref() == Some("40001")
    )
}
