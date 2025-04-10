//! Errors specific to the main client orchestration or launch logic.

use janus_core::error::CoreError;
use janus_interfaces::ApiError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Configuration loading failed: {0}")]
    ConfigError(#[from] CoreError),

    #[error("Actor system failed to start: {0}")]
    ActorSystemError(String),

    #[error("Browser launch failed: {0}")]
    LaunchError(String),

    #[error("Supervisor actor failed: {0}")]
    SupervisorError(String),

    #[error("Unsupported browser type specified")]
    UnsupportedBrowser,

    // Can wrap ApiError if a launch process results in a connect error etc.
    #[error(transparent)]
    Api(#[from] ApiError),
}

// Optional: Convert ClientError to ApiError for the final user-facing result
impl From<ClientError> for ApiError {
    fn from(err: ClientError) -> Self {
        match err {
            ClientError::ConfigError(e) => ApiError::InternalError(format!("Config error: {}", e)),
            ClientError::ActorSystemError(e) => {
                ApiError::InternalError(format!("Actor system: {}", e))
            }
            ClientError::LaunchError(e) => ApiError::LaunchError(e),
            ClientError::SupervisorError(e) => {
                ApiError::InternalError(format!("Supervisor: {}", e))
            }
            ClientError::UnsupportedBrowser => {
                ApiError::InvalidParameters("Unsupported browser type".into())
            }
            ClientError::Api(api_err) => api_err, // Pass through existing ApiErrors
        }
    }
}
