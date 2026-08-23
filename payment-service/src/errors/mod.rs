use std::{error::Error, fmt};

/// Errors encountered while reading startup configuration.
#[derive(Debug)]
pub enum ConfigError {
    InvalidServerPort,
    InvalidServerAddress,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidServerPort => formatter.write_str("SERVER_PORT must be a valid u16"),
            Self::InvalidServerAddress => {
                formatter.write_str("SERVER_HOST and SERVER_PORT must form a valid socket address")
            }
        }
    }
}

impl Error for ConfigError {}
