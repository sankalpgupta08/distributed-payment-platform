//! Database boundary for the Payment Service.
//!
//! Connection-pool setup is introduced in Phase 3. Phase 4 owns the versioned
//! schema migrations. SQLx repositories are added when payment persistence
//! begins in Phase 5.

pub mod connection;
pub mod migrations;
