use async_trait::async_trait;
use serde_json::Value;
use thiserror::Error;

// --- Error Types ---
#[derive(Error, Debug, Clone, PartialEq)]
pub enum TransportError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("TLS error: {0}")]
    TlsError(String),
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    #[error("Connection timeout: {0}")]
    Timeout(String),
    #[error("Connection closed: {reason}")]
    ConnectionClosed { reason: String },
    #[error("Not connected")]
    NotConnected,
    #[error("WebSocket error: {0}")]
    WebSocket(String),
    #[error("Serialization error: {0}")]
    Serde(String),
    #[error("Unsupported scheme: {0}")]
    UnsupportedScheme(String),
    #[error("Send failed: {0}")]
    SendFailed(String),
    #[error("Receive failed: {0}")]
    ReceiveFailed(String),
    #[error("I/O error: {0}")]
    Io(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("Operation timed out")]
    Timeout,
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    #[error("Browser error: {message} (code: {code})")]
    BrowserError { code: i32, message: String },
    #[error("Response parse error: {reason} (method: {method})")]
    ResponseParseError { method: String, reason: String },
    #[error("Event parse error: {reason} (event: {event})")]
    EventParseError { event: String, reason: String },
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Target or session not found: {0}")]
    TargetOrSessionNotFound(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Transport error: {0}")]
    Transport(#[from] TransportError),
    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Actor system error: {0}")]
    ActorSystem(String),
    #[error("Actor mailbox error: {0}")]
    ActorMailbox(String),
    #[error("Resource initialization error: {0}")]
    ResourceInitialization(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

// --- Transport Module ---
mod transport;
pub use transport::*;

// --- Actor Module ---
mod actor;
pub use actor::*;

// --- Config Module ---
mod config;
pub use config::*;

// --- Browser Interface Types ---
#[derive(Debug, Clone)]
pub struct ElementHandle { /* Opaque handle representation */ pub internal_id: String }
#[derive(Debug, Clone)]
pub struct ConsoleMessage { /* Details of console message */ pub text: String }
#[derive(Debug, Clone)]
pub enum ScreenshotFormat { Jpeg, Png, Webp }
#[derive(Debug, Clone, Default)]
pub struct ScreenshotOptions { /* Quality, clip rect etc. */ pub quality: Option<u8> }
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubscriptionId(pub u64); // Example simple subscription ID

// --- L1 API Error Type ---
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Operation timed out")]
    Timeout,
    #[error("Protocol error: {0}")]
    ProtocolError(String),
    #[error("Browser process crashed or closed unexpectedly")]
    BrowserCrashed,
    #[error("Invalid parameters provided: {0}")]
    InvalidParameters(String),
    #[error("Target (e.g., Page) not found or closed")]
    TargetNotFound,
    #[error("Navigation failed: {0}")]
    NavigationError(String),
    #[error("Script execution failed: {0}")]
    ScriptError(String),
    #[error("DOM operation failed: {0}")]
    DomError(String),
    #[error("Feature not supported by this browser/protocol")]
    NotSupported,
    #[error("Internal client error: {0}")]
    InternalError(String),
}

// --- Conversion from internal errors to public API error ---
impl From<CoreError> for ApiError {
    fn from(err: CoreError) -> Self {
        match err {
            CoreError::Transport(t_err) => match t_err {
                TransportError::ConnectionFailed(reason) |
                TransportError::TlsError(reason) |
                TransportError::InvalidUrl(reason) => ApiError::ConnectionFailed(reason),
                TransportError::Timeout(reason) => ApiError::ConnectionFailed(format!("Timeout during connection: {}", reason)),
                TransportError::ConnectionClosed { .. } => ApiError::BrowserCrashed,
                TransportError::NotConnected => ApiError::ConnectionFailed("Not connected".to_string()),
                TransportError::WebSocket(reason) |
                TransportError::Serde(reason) => ApiError::ProtocolError(reason),
                TransportError::UnsupportedScheme(scheme) => ApiError::ConnectionFailed(format!("Unsupported protocol scheme: {}", scheme)),
                TransportError::SendFailed(reason) |
                TransportError::ReceiveFailed(reason) => ApiError::ProtocolError(format!("Message transport failed: {}", reason)),
                TransportError::Io(reason) => ApiError::ConnectionFailed(format!("Network I/O error: {}", reason)),
                TransportError::Internal(reason) => ApiError::InternalError(format!("Transport layer internal error: {}", reason)),
            },
            CoreError::Protocol(p_err) => match p_err {
                ProtocolError::Timeout => ApiError::Timeout,
                ProtocolError::InvalidRequest(reason) => ApiError::InvalidParameters(reason),
                ProtocolError::BrowserError { message, .. } => ApiError::ProtocolError(message),
                ProtocolError::ResponseParseError { reason, .. } |
                ProtocolError::EventParseError { reason, .. } |
                ProtocolError::SerializationError(reason) => ApiError::ProtocolError(format!("Protocol serialization/parsing error: {}", reason)),
                ProtocolError::TargetOrSessionNotFound(_) => ApiError::TargetNotFound,
                ProtocolError::Internal(reason) => ApiError::InternalError(format!("Protocol layer internal error: {}", reason)),
            },
            CoreError::Config(cfg_err) => ApiError::InternalError(format!("Configuration error: {}", cfg_err)),
            CoreError::ActorSystem(reason) |
            CoreError::Internal(reason) => ApiError::InternalError(reason),
            CoreError::ActorMailbox(mb_err) => ApiError::InternalError(format!("Internal communication error: {}", mb_err)),
            CoreError::ResourceInitialization(reason) => ApiError::ConnectionFailed(format!("Failed to initialize browser resource: {}", reason)),
        }
    }
}

// --- L1 Browser Trait ---
#[async_trait]
pub trait Browser: Send + Sync {
    async fn disconnect(&mut self) -> Result<(), ApiError>;
    async fn close(&mut self) -> Result<(), ApiError>;
    async fn new_page(&self) -> Result<Box<dyn Page>, ApiError>;
    async fn pages(&self) -> Result<Vec<Box<dyn Page>>, ApiError>;
    async fn version(&self) -> Result<String, ApiError>;
}

// --- L1 Page Trait ---
#[async_trait]
pub trait Page: Send + Sync {
    async fn navigate(&self, url: &str) -> Result<(), ApiError>;
    async fn reload(&self) -> Result<(), ApiError>;
    async fn go_back(&self) -> Result<(), ApiError>;
    async fn go_forward(&self) -> Result<(), ApiError>;
    async fn close(&self) -> Result<(), ApiError>;
    fn id(&self) -> String;
    async fn content(&self) -> Result<String, ApiError>;
    async fn evaluate_script(&self, script: &str) -> Result<Value, ApiError>;
    async fn call_function(&self, function_declaration: &str, args: Vec<Value>) -> Result<Value, ApiError>;
    async fn query_selector(&self, selector: &str) -> Result<Option<ElementHandle>, ApiError>;
    async fn wait_for_selector(&self, selector: &str, timeout_ms: Option<u64>) -> Result<ElementHandle, ApiError>;
    async fn url(&self) -> Result<String, ApiError>;
    async fn title(&self) -> Result<String, ApiError>;
    async fn take_screenshot(&self, format: ScreenshotFormat, options: Option<ScreenshotOptions>) -> Result<Vec<u8>, ApiError>;
}

mod error;
pub use error::*;

pub use janus_interface;
pub use janus_transport;
