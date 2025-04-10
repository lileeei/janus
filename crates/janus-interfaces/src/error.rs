use thiserror::Error;

/// Represents common high-level errors surfaced to the user of the Janus client API.
/// These errors are intended to be protocol-agnostic.
#[derive(Error, Debug)]
pub enum ApiError {
    /// Failed to establish or maintain a connection to the browser's debugging endpoint.
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// An operation did not complete within the specified or default timeout period.
    #[error("Operation timed out")]
    Timeout,

    /// An error occurred related to the debugging protocol itself (e.g., malformed message,
    /// unexpected response, command rejected by the browser). Contains details from the
    /// underlying protocol error if available.
    #[error("Protocol error: {0}")]
    ProtocolError(String),

    /// The browser process unexpectedly terminated or crashed.
    #[error("Browser process crashed or closed unexpectedly")]
    BrowserCrashed,

    /// Invalid parameters were provided to an API method.
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    /// The requested browser feature or operation is not supported by the current
    /// browser instance or protocol implementation.
    #[error("Operation not supported: {0}")]
    NotSupported(String),

    /// The target (e.g., Page, Worker) associated with an operation no longer exists.
    #[error("Target detached or closed")]
    TargetDetached,

    /// An internal error occurred within the Janus client library. This may indicate a bug.
    #[error("Internal client error: {0}")]
    InternalError(String),

    /// Error related to launching the browser process.
    #[error("Failed to launch browser: {0}")]
    LaunchError(String),

    /// Generic I/O error occurred.
    #[error("I/O error: {0}")]
    IoError(String),
    // Consider adding more specific common errors as needed, e.g., NavigationFailed
}

// Allow easy conversion from IO errors if needed at the API boundary
impl From<std::io::Error> for ApiError {
    fn from(err: std::io::Error) -> Self {
        ApiError::IoError(err.to_string())
    }
}
