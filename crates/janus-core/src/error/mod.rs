// JanusClient/janus-client/crates/janus-core/src/error/mod.rs
use thiserror::Error;
use std::time::Duration; // Needed if timeout details are included

// Re-export for convenience elsewhere
pub use config::ConfigError;
pub use actix::MailboxError;

// --- Transport Error (L3) ---
#[derive(Error, Debug, Clone)] // Clone might be useful for some scenarios, e.g., state reporting
pub enum TransportError {
    #[error("Invalid URL format: {0}")]
    InvalidUrl(String),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Connection closed: {reason:?}")]
    ConnectionClosed { reason: Option<String> },

    #[error("Not connected")]
    NotConnected,

    #[error("I/O error: {0}")]
    Io(String), // Wrap std/tokio IO error strings

    #[error("TLS error: {0}")]
    TlsError(String),

    #[error("WebSocket protocol error: {0}")]
    WebSocket(String), // Wrap tungstenite::Error strings (protocol related)

    #[error("Failed to send message: {0}")]
    SendFailed(String),

    #[error("Failed to receive message: {0}")]
    ReceiveFailed(String),

    #[error("Operation timed out: {0}")]
    Timeout(String), // Specific timeout details

    #[error("Serialization/Deserialization error: {0}")]
    Serde(String), // e.g., invalid UTF8, framing issues

    #[error("Unsupported URL scheme: {0}")]
    UnsupportedScheme(String),

    #[error("Internal transport error: {0}")]
    Internal(String),
}

// --- Protocol Error (L2/Core Interaction) ---
#[derive(Error, Debug, Clone)] // Clone might be useful
pub enum ProtocolError {
    #[error("Invalid command parameters: {0}")]
    InvalidRequest(String),

    #[error("Browser returned error: code={code}, message='{message}', data={data:?}")]
    BrowserError {
        code: i64,
        message: String,
        data: Option<serde_json::Value>,
    },

    #[error("Failed to parse browser response: {reason} - Response fragment: '{response_fragment}'")]
    ResponseParseError {
        reason: String,
        response_fragment: String, // Include part of the problematic response
    },

    #[error("Failed to parse browser event: {reason} - Event fragment: '{event_fragment}'")]
    EventParseError {
        reason: String,
        event_fragment: String, // Include part of the problematic event
    },

    #[error("Waiting for command response timed out")]
    Timeout, // Specific to waiting for a protocol response

    #[error("Target or session not found: {0}")]
    TargetOrSessionNotFound(String),

    #[error("Failed to serialize command: {0}")]
    SerializationError(String),

    #[error("Internal protocol handling error: {0}")]
    Internal(String),
}


// --- Core Error (Top-level Internal) ---
#[derive(Error, Debug)] // Avoid cloning CoreError unless necessary, can contain non-Clone types
pub enum CoreError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Actor system error: {0}")]
    ActorSystem(String), // Generic actor issues

    #[error("Actor mailbox error: {0}")]
    ActorMailbox(#[from] MailboxError),

    #[error("Transport layer error")]
    Transport(#[from] TransportError),

    #[error("Protocol layer error")]
    Protocol(#[from] ProtocolError),

    #[error("Resource initialization failed: {0}")]
    ResourceInitialization(String), // e.g., browser launch failed

    #[error("Internal client error: {0}")]
    Internal(String),
}
