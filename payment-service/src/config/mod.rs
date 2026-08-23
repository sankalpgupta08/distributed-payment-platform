use std::{env, net::SocketAddr};

use crate::errors::ConfigError;

/// Configuration required to start the HTTP service and connect to PostgreSQL.
///
/// Redis settings will be added in a later phase.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub server_host: String,
    pub server_port: u16,
    pub database_url: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let server_host = env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
        let server_port = env::var("SERVER_PORT")
            .unwrap_or_else(|_| "3000".to_owned())
            .parse::<u16>()
            .map_err(|_| ConfigError::InvalidServerPort)?;
        let database_url = env::var("DATABASE_URL").map_err(|_| ConfigError::MissingDatabaseUrl)?;

        Ok(Self {
            server_host,
            server_port,
            database_url,
        })
    }

    pub fn socket_addr(&self) -> Result<SocketAddr, ConfigError> {
        format!("{}:{}", self.server_host, self.server_port)
            .parse()
            .map_err(|_| ConfigError::InvalidServerAddress)
    }
}
