use axum::{
    Json,
    extract::{
        Path, State,
        rejection::{JsonRejection, PathRejection},
    },
    http::StatusCode,
};
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::{
        payment::Payment,
        requests::{CreatePaymentRequest, UpdatePaymentStatusRequest},
    },
    services::payment_service,
    state::AppState,
};

/// Creates a durable payment record with an initial `pending` status.
pub async fn create_payment(
    State(state): State<AppState>,
    payload: Result<Json<CreatePaymentRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<Payment>), AppError> {
    let Json(request) =
        payload.map_err(|rejection| AppError::bad_request(rejection.body_text()))?;
    let payment = payment_service::create_payment(&state.db, request).await?;

    Ok((StatusCode::CREATED, Json(payment)))
}

/// Returns the current persisted state of one payment.
pub async fn get_payment(
    State(state): State<AppState>,
    payment_id: Result<Path<Uuid>, PathRejection>,
) -> Result<Json<Payment>, AppError> {
    let Path(payment_id) =
        payment_id.map_err(|_| AppError::bad_request("payment id must be a valid UUID"))?;
    let payment = payment_service::get_payment(&state.db, payment_id).await?;

    Ok(Json(payment))
}

/// Advances one payment to a permitted next lifecycle status.
pub async fn update_payment_status(
    State(state): State<AppState>,
    payment_id: Result<Path<Uuid>, PathRejection>,
    payload: Result<Json<UpdatePaymentStatusRequest>, JsonRejection>,
) -> Result<Json<Payment>, AppError> {
    let Path(payment_id) =
        payment_id.map_err(|_| AppError::bad_request("payment id must be a valid UUID"))?;
    let Json(request) =
        payload.map_err(|rejection| AppError::bad_request(rejection.body_text()))?;
    let payment = payment_service::update_payment_status(&state.db, payment_id, request).await?;

    Ok(Json(payment))
}
