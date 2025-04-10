use thiserror::Error;

/// Errors specific to the transport layer (L3).
#[derive(Error, Debug, Clone)] // Clone might be useful if error needs to be stored/passed
pub enum TransportError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Not connected: {0}")]
    NotConnected(String),

    #[error("Send operation failed: {0}")]
    SendFailed(String),

    #[error("Receive operation failed: {0}")]
    ReceiveFailed(String),

    #[error("Serialization/Deserialization error (Transport Level): {0}")]
    SerdeError(String), // For framing errors, etc.

    #[error("Connection timed out")]
    Timeout,

    #[error("Invalid URL or connection parameters: {0}")]
    InvalidUrl(String),

    #[error("Unsupported URL scheme: {0}")]
    UnsupportedScheme(String),

    #[error("Underlying I/O error: {0}")]
    Io(String), // Wrap std::io::Error string representation

    #[cfg(feature = "websocket")]
    #[error("WebSocket protocol error: {0}")]
    WebSocketError(String),

    #[error("TLS error: {0}")]
    TlsError(String),

    #[error("Operation cancelled")]
    Cancelled,

    #[error("Unknown transport error: {0}")]
    Other(String),
}

// Helper for converting std::io::Error
impl From<std::io::Error> for TransportError {
    fn from(err: std::io::Error) -> Self {
        TransportError::Io(err.to_string())
    }
}

// Add From implementations for tungstenite errors if websocket feature is enabled
#[cfg(feature = "websocket")]
impl From<tokio_tungstenite::tungstenite::Error> for TransportError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        match err {
            tokio_tungstenite::tungstenite::Error::ConnectionClosed => {
                TransportError::NotConnected("Connection closed".into())
            }
            tokio_tungstenite::tungstenite::Error::AlreadyClosed => {
                TransportError::NotConnected("Connection already closed".into())
            }
            tokio_tungstenite::tungstenite::Error::Io(io_err) => {
                TransportError::Io(io_err.to_string())
            }
            tokio_tungstenite::tungstenite::Error::Tls(tls_err) => {
                // The exact error type might be behind feature flags in tungstenite itself
                TransportError::TlsError(format!("TLS Error: {:?}", tls_err))
            }
            tokio_tungstenite::tungstenite::Error::Capacity(reason) => {
                TransportError::SendFailed(format!("Capacity error: {}", reason))
            }
            tokio_tungstenite::tungstenite::Error::Protocol(reason) => {
                TransportError::WebSocketError(format!("Protocol violation: {}", reason))
            }
            tokio_tungstenite::tungstenite::Error::SendQueueFull => {
                TransportError::SendFailed("Send queue full".into())
            }
            tokio_tungstenite::tungstenite::Error::Utf8 => {
                TransportError::ReceiveFailed("Invalid UTF-8 received".into())
            }
            tokio_tungstenite::tungstenite::Error::Url(parse_err) => {
                TransportError::InvalidUrl(format!("URL parse error: {}", parse_err))
            }
            tokio_tungstenite::tungstenite::Error::Http(resp) => TransportError::ConnectionFailed(
                format!("HTTP error during handshake: Status {}", resp.status()),
            ),
            tokio_tungstenite::tungstenite::Error::HttpFormat(http_err) => {
                TransportError::ConnectionFailed(format!("HTTP format error: {}", http_err))
            }
            // Handle _ variants if necessary based on tungstenite version
            _ => TransportError::Other(format!("Unknown tungstenite error: {}", err)),
        }
    }
}
