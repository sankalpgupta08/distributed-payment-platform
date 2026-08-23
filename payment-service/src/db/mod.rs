//! Database boundary for the Payment Service.
//!
//! The connection pool and SQLx repositories are intentionally introduced in
//! Phase 3, keeping this architecture phase free of persistence concerns.

pub mod connection;
