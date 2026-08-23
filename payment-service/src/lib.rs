//! Reusable application modules for the Payment Service.
//!
//! `main.rs` is intentionally limited to application startup. HTTP routing,
//! request handlers, configuration, and shared dependencies live here so they
//! can be tested and extended independently.

pub mod config;
pub mod db;
pub mod errors;
pub mod handlers;
pub mod models;
pub mod routes;
pub mod state;
