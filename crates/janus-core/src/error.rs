use thiserror::Error;

/// Represents errors originating from within the Janus client's internal logic,
/// distinct from the high-level `ApiError` exposed to users.
#[derive(Error, Debug)]
pub enum InternalError {
    /// An error occurred in the transport layer (e.g., WebSocket, TCP).
    #[error("Transport error: {0}")]
    Transport(String), // Transport error represented as a String

    /// An error related to the browser's debugging protocol itself.
    /// This often wraps specific protocol error details.
    #[error("Protocol error: {message}")]
    Protocol {
        code: Option<i64>,    // Protocol-specific error code (e.g., JSON-RPC error code)
        message: String,      // Error message from the protocol/browser
        data: Option<String>, // Optional additional data as string
    },

    /// An error occurred within the actor system (e.g., mailbox full, actor panicked).
    #[error("Actor system error: {0}")]
    Actor(String), // Keep simple for now, maybe use specific actor error types later

    /// An operation timed out internally.
    #[error("Internal operation timed out")]
    Timeout,

    /// Could not determine the state or details of the browser process (likely crashed).
    #[error("Browser process died or is unresponsive")]
    BrowserProcessDied,

    /// Invalid parameters were detected internally.
    #[error("Invalid internal parameters: {0}")]
    InvalidParams(String),

    /// Failed to serialize data (e.g., command parameters).
    #[error("Serialization failed: {0}")]
    Serialization(String),

    /// Failed to deserialize data (e.g., responses, events).
    #[error("Deserialization failed: {0}")]
    Deserialization(String),

    /// A required configuration value was missing or invalid.
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Core internal error, potentially a bug.
    #[error("Core internal error: {0}")]
    Core(#[from] CoreError),
}

/// Specific errors originating strictly from the core crate logic.
#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Configuration loading failed: {0}")]
    ConfigLoad(#[from] config::ConfigError),

    #[error("Logging setup failed: {0}")]
    LoggingSetup(String),
}

// Conversion from CoreError to InternalError is handled by derive(Error)

// Allow easy conversion from serde_json errors
impl From<serde_json::Error> for InternalError {
    fn from(err: serde_json::Error) -> Self {
        // Differentiate based on context if possible, otherwise use generic variants
        if err.is_data() || err.is_syntax() {
            InternalError::Deserialization(err.to_string())
        } else {
            InternalError::Serialization(err.to_string()) // Or Deserialization depending on context
        }
    }
}
