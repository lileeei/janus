use thiserror::Error;
use janus_interface::transport::TransportError;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Transport error: {0}")]
    Transport(#[from] TransportError),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Protocol error: {0}")]
    Protocol(String),
    
    #[error("Actor mailbox error: {0}")]
    Mailbox(String),
    
    #[error("Browser error: {0}")]
    Browser(String),
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<std::io::Error> for CoreError {
    fn from(err: std::io::Error) -> Self {
        CoreError::Unknown(err.to_string())
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to load config: {0}")]
    LoadError(String),
    
    #[error("Invalid config value: {0}")]
    InvalidValue(String),
    
    #[error("Missing required config: {0}")]
    MissingValue(String),
} 