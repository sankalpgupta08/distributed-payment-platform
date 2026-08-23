use rust_decimal::Decimal;
use serde::Deserialize;
use uuid::Uuid;

use crate::{errors::AppError, models::payment::NewPayment};

/// JSON accepted by `POST /payments`.
#[derive(Debug, Deserialize)]
pub struct CreatePaymentRequest {
    pub merchant_id: Uuid,
    pub amount: Decimal,
    pub currency: String,
}

impl CreatePaymentRequest {
    /// Validates client-controlled fields before any database query is made.
    pub fn into_new_payment(self, id: Uuid) -> Result<NewPayment, AppError> {
        if self.amount <= Decimal::ZERO {
            return Err(AppError::bad_request("amount must be greater than zero"));
        }

        if self.amount.scale() > 2 {
            return Err(AppError::bad_request(
                "amount must have no more than two decimal places",
            ));
        }

        if !is_valid_currency(&self.currency) {
            return Err(AppError::bad_request(
                "currency must be a three-letter uppercase ISO code",
            ));
        }

        Ok(NewPayment {
            id,
            merchant_id: self.merchant_id,
            amount: self.amount,
            currency: self.currency,
            status: crate::models::payment::PaymentStatus::Pending,
        })
    }
}

fn is_valid_currency(currency: &str) -> bool {
    currency.len() == 3 && currency.bytes().all(|byte| byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use uuid::Uuid;

    use super::CreatePaymentRequest;

    #[test]
    fn rejects_more_than_two_decimal_places() {
        let request = CreatePaymentRequest {
            merchant_id: Uuid::new_v4(),
            amount: Decimal::new(123, 3),
            currency: "INR".to_owned(),
        };

        assert!(request.into_new_payment(Uuid::new_v4()).is_err());
    }

    #[test]
    fn accepts_a_valid_request() {
        let request = CreatePaymentRequest {
            merchant_id: Uuid::new_v4(),
            amount: Decimal::new(50000, 2),
            currency: "INR".to_owned(),
        };

        assert!(request.into_new_payment(Uuid::new_v4()).is_ok());
    }
}
