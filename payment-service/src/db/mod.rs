//! Database boundary for the Payment Service.
//!
//! Phase 3 owns connection-pool setup. SQLx repositories are added when payment
//! persistence begins in Phase 5.

pub mod connection;
