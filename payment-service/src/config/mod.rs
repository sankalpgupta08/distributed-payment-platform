use std::{env, net::SocketAddr, time::Duration};

use crate::errors::ConfigError;

/// Configuration required to run the HTTP service and its durable dependencies.
#[derive(Clone)]
pub struct AppConfig {
    pub server_host: String,
    pub server_port: u16,
    pub database_url: String,
    pub database_max_connections: u32,
    pub redis_url: String,
    pub redis_lock_ttl: Duration,
    pub idempotency_ttl: Duration,
    pub request_timeout: Duration,
    pub max_request_body_bytes: usize,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let server_host = env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
        let server_port = env::var("SERVER_PORT")
            .unwrap_or_else(|_| "3000".to_owned())
            .parse::<u16>()
            .map_err(|_| ConfigError::InvalidServerPort)?;
        let database_url = env::var("DATABASE_URL").map_err(|_| ConfigError::MissingDatabaseUrl)?;
        let database_max_connections = positive_env("DATABASE_MAX_CONNECTIONS", 10)? as u32;
        let redis_url = env::var("REDIS_URL").map_err(|_| ConfigError::MissingRedisUrl)?;
        let redis_lock_ttl = Duration::from_secs(positive_env("REDIS_LOCK_TTL_SECONDS", 30)?);
        let idempotency_ttl = Duration::from_secs(positive_env("IDEMPOTENCY_TTL_SECONDS", 86_400)?);
        let request_timeout = Duration::from_secs(positive_env("REQUEST_TIMEOUT_SECONDS", 10)?);
        let max_request_body_bytes = positive_env("MAX_REQUEST_BODY_BYTES", 16_384)? as usize;

        Ok(Self {
            server_host,
            server_port,
            database_url,
            database_max_connections,
            redis_url,
            redis_lock_ttl,
            idempotency_ttl,
            request_timeout,
            max_request_body_bytes,
        })
    }

    pub fn socket_addr(&self) -> Result<SocketAddr, ConfigError> {
        format!("{}:{}", self.server_host, self.server_port)
            .parse()
            .map_err(|_| ConfigError::InvalidServerAddress)
    }
}

fn positive_env(variable: &'static str, default: u64) -> Result<u64, ConfigError> {
    let value = env::var(variable).unwrap_or_else(|_| default.to_string());
    let parsed = value
        .parse::<u64>()
        .map_err(|_| ConfigError::InvalidPositiveInteger(variable))?;

    if parsed == 0 {
        return Err(ConfigError::InvalidPositiveInteger(variable));
    }

    Ok(parsed)
}
