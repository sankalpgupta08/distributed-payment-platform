use std::{error::Error, fmt};

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

/// Errors encountered while reading startup configuration.
#[derive(Debug)]
pub enum ConfigError {
    InvalidServerPort,
    InvalidServerAddress,
    MissingDatabaseUrl,
    MissingRedisUrl,
    InvalidPositiveInteger(&'static str),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidServerPort => formatter.write_str("SERVER_PORT must be a valid u16"),
            Self::InvalidServerAddress => {
                formatter.write_str("SERVER_HOST and SERVER_PORT must form a valid socket address")
            }
            Self::MissingDatabaseUrl => formatter.write_str("DATABASE_URL must be set"),
            Self::MissingRedisUrl => formatter.write_str("REDIS_URL must be set"),
            Self::InvalidPositiveInteger(variable) => {
                write!(formatter, "{variable} must be a positive integer")
            }
        }
    }
}

impl Error for ConfigError {}

/// An application failure that can be safely translated into an HTTP response.
#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    NotFound(String),
    Conflict(String),
    Internal,
}

impl AppError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest(message.into())
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict(message.into())
    }
}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        tracing::error!(error = %error, "database operation failed");
        Self::Internal
    }
}

impl From<redis::RedisError> for AppError {
    fn from(error: redis::RedisError) -> Self {
        tracing::error!(error = %error, "Redis operation failed");
        Self::Internal
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    error: &'static str,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error, message) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, "bad_request", message),
            Self::NotFound(message) => (StatusCode::NOT_FOUND, "not_found", message),
            Self::Conflict(message) => (StatusCode::CONFLICT, "conflict", message),
            Self::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_server_error",
                "an unexpected error occurred".to_owned(),
            ),
        };

        (status, Json(ErrorResponse { error, message })).into_response()
    }
}
