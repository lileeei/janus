use thiserror::Error;
use janus_core::error::CoreError;

#[derive(Debug, Error)]
pub enum ChromeError {
    #[error("Failed to launch Chrome browser: {0}")]
    LaunchError(String),

    #[error("Chrome process error: {0}")]
    ProcessError(String),

    #[error("DevTools protocol error: {0}")]
    ProtocolError(String),

    #[error("Page error: {0}")]
    PageError(String),

    #[error("Target not found: {0}")]
    TargetNotFound(String),

    #[error("Session error: {0}")]
    SessionError(String),

    #[error("Timeout error: {0}")]
    TimeoutError(String),

    #[error(transparent)]
    CoreError(#[from] CoreError),

    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

impl From<ChromeError> for CoreError {
    fn from(err: ChromeError) -> Self {
        match err {
            ChromeError::LaunchError(msg) | ChromeError::ProcessError(msg) => {
                CoreError::ResourceInitialization(msg)
            }
            ChromeError::ProtocolError(msg) => {
                CoreError::Protocol(janus_core::error::ProtocolError::CommandError { reason: msg })
            }
            ChromeError::PageError(msg) | ChromeError::TargetNotFound(msg) => {
                CoreError::ResourceNotFound(msg)
            }
            ChromeError::SessionError(msg) => {
                CoreError::Protocol(janus_core::error::ProtocolError::SessionError { reason: msg })
            }
            ChromeError::TimeoutError(msg) => CoreError::Timeout(msg),
            ChromeError::CoreError(err) => err,
            ChromeError::IoError(err) => CoreError::IoError(err.to_string()),
        }
    }
} 