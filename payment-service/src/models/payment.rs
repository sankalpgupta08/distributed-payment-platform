use std::{error::Error, fmt};

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The allowed lifecycle states of a payment.
///
/// Phase 6 will add rules that control which transitions are valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentStatus {
    Pending,
    Processing,
    Succeeded,
    Failed,
}

impl PaymentStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Processing => "processing",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }

    /// Returns whether one lifecycle state may move directly to another.
    pub const fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Pending, Self::Processing) | (Self::Processing, Self::Succeeded | Self::Failed)
        )
    }
}

impl fmt::Display for PaymentStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug)]
pub struct PaymentStatusParseError(String);

impl fmt::Display for PaymentStatusParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "unknown payment status: {}", self.0)
    }
}

impl Error for PaymentStatusParseError {}

impl TryFrom<&str> for PaymentStatus {
    type Error = PaymentStatusParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "pending" => Ok(Self::Pending),
            "processing" => Ok(Self::Processing),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            _ => Err(PaymentStatusParseError(value.to_owned())),
        }
    }
}

/// The durable payment record represented by the `payments` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub id: Uuid,
    pub merchant_id: Uuid,
    pub amount: Decimal,
    pub currency: String,
    pub status: PaymentStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Data required to insert a newly-created payment.
#[derive(Debug, Clone)]
pub struct NewPayment {
    pub id: Uuid,
    pub merchant_id: Uuid,
    pub amount: Decimal,
    pub currency: String,
    pub status: PaymentStatus,
}

#[cfg(test)]
mod tests {
    use super::PaymentStatus;

    #[test]
    fn parses_known_database_status() {
        assert_eq!(
            PaymentStatus::try_from("succeeded").unwrap(),
            PaymentStatus::Succeeded
        );
    }

    #[test]
    fn rejects_unknown_database_status() {
        assert!(PaymentStatus::try_from("cancelled").is_err());
    }

    #[test]
    fn permits_only_defined_lifecycle_transitions() {
        assert!(PaymentStatus::Pending.can_transition_to(PaymentStatus::Processing));
        assert!(PaymentStatus::Processing.can_transition_to(PaymentStatus::Succeeded));
        assert!(PaymentStatus::Processing.can_transition_to(PaymentStatus::Failed));

        assert!(!PaymentStatus::Pending.can_transition_to(PaymentStatus::Succeeded));
        assert!(!PaymentStatus::Succeeded.can_transition_to(PaymentStatus::Failed));
        assert!(!PaymentStatus::Failed.can_transition_to(PaymentStatus::Processing));
    }
}
