use std::{error::Error, fmt};

/// Errors encountered while reading startup configuration.
#[derive(Debug)]
pub enum ConfigError {
    InvalidServerPort,
    InvalidServerAddress,
    MissingDatabaseUrl,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidServerPort => formatter.write_str("SERVER_PORT must be a valid u16"),
            Self::InvalidServerAddress => {
                formatter.write_str("SERVER_HOST and SERVER_PORT must form a valid socket address")
            }
            Self::MissingDatabaseUrl => formatter.write_str("DATABASE_URL must be set"),
        }
    }
}

impl Error for ConfigError {}
