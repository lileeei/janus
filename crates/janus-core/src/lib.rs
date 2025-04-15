//! # Janus Core
//!
//! This crate provides core utilities, shared types, configuration handling,
//! internal error definitions, and potentially base actor functionalities
//! used by other Janus crates.

pub mod config;
pub mod error;
pub mod logging; // Optional logging setup helper

// Re-export key items for easier use by other crates
pub use config::{load_config, Config};
pub use error::InternalError;
// Re-export TransportError if it lives here, otherwise it's in janus-transport
// pub use error::TransportError;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert_eq!(cfg.global.log_level, "info");
        assert_eq!(cfg.global.default_command_timeout, Duration::from_secs(30));
        assert_eq!(cfg.transport.connect_timeout, Duration::from_secs(20));
        #[cfg(feature = "websocket")]
        assert_eq!(cfg.transport.websocket.accept_unmasked_frames, false);
    }

    // Basic test to ensure config loading structure works (doesn't actually load files)
    #[test]
    fn test_load_config_structure() {
        // We can't easily test file/env loading in unit tests,
        // but we can check if the builder pattern works.
        let builder = ::config::Config::builder()
            .set_default("global.log_level", "debug")
            .unwrap()
            .set_default("global.default_command_timeout_ms", 10000)
            .unwrap();

        let result: Result<Config, ::config::ConfigError> =
            builder.build().unwrap().try_deserialize();
        assert!(result.is_ok());
        let cfg = result.unwrap();
        assert_eq!(cfg.global.log_level, "debug");
        assert_eq!(
            cfg.global.default_command_timeout,
            Duration::from_millis(10000)
        );
    }
}
