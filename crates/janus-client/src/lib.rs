//! # Janus Client Library
//!
//! This crate provides the main entry point and orchestration logic for the
//! Janus client. It ties together the interface, core, transport, protocol handling,
//! and browser-specific implementation crates.
//!
//! Users typically interact with this crate to launch browsers and obtain
//! handles (`Browser`, `Page`) defined in `janus-interfaces`.

// Re-export the L1 API for user convenience
pub use janus_interfaces::{
    ApiError,
    Browser,
    Clip,
    // Re-export other common types as needed
    ConsoleLogLevel,
    ConsoleMessage,
    ElementHandle,
    Page,
    ScreenshotFormat,
    ScreenshotOptions,
    SubscriptionId,
};

// Export value for JSON parsing/serialization
pub use serde_json::Value;

// Re-export core types if needed by users (e.g. Config for advanced setup)
pub use janus_core::Config; // Make Config accessible

// Re-export specific Transport types if needed for advanced config/launch
pub use janus_transport::{ConnectParams, WebSocketConnectOptions};

// Modules internal to this crate
mod error;
mod launch; // Placeholder for launch functions
mod supervisor; // Placeholder for the main supervisor

pub use error::ClientError;
pub use launch::launch; // Example basic launch function

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    // Test re-exports (compile-time check)
    #[allow(dead_code)]
    fn check_reexports() {
        let _e: ApiError = ApiError::Timeout;
        let _cfg: Config = Config::default();
        let _params: ConnectParams = ConnectParams {
            url: String::new(),
            connection_timeout: std::time::Duration::from_secs(1),
            // #[cfg(feature = "websocket")]
            ws_options: WebSocketConnectOptions::default(),
        };
        // Cannot instantiate traits directly
        // let _b: Box<dyn Browser>;
        // let _p: Box<dyn Page>;
    }
}
