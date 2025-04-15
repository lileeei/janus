//! Error mapping for Chrome implementation

use janus_core::error::InternalError;
use janus_interfaces::ApiError;

// Helper function to map internal errors (Actor/Protocol/Transport) to public ApiError
pub(crate) fn map_internal_to_api_error(internal_error: InternalError) -> ApiError {
    match internal_error {
        InternalError::Transport(transport_err) => {
            ApiError::ConnectionFailed(transport_err)
        }
        InternalError::Protocol { message, .. } => ApiError::ProtocolError(message), // Simplify for now
        InternalError::Actor(actor_err) => {
            ApiError::InternalError(format!("Internal actor error: {}", actor_err))
        }
        InternalError::Timeout => ApiError::Timeout,
        InternalError::BrowserProcessDied => ApiError::BrowserCrashed,
        InternalError::InvalidParams(msg) => ApiError::InvalidParameters(msg),
        InternalError::Serialization(msg) | InternalError::Deserialization(msg) => {
            ApiError::InternalError(format!("Serialization error: {}", msg))
        }
        InternalError::Configuration(msg) => {
            ApiError::InternalError(format!("Configuration error: {}", msg))
        }
        InternalError::Core(core_err) => {
            ApiError::InternalError(format!("Core error: {}", core_err))
        }
    }
}
