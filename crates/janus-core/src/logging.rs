//! Optional helper for setting up logging using `env_logger`.

use crate::error::CoreError;

#[cfg(feature = "env_logger")]
pub fn setup_logging(log_level_str: &str) -> Result<(), CoreError> {
    use env_logger::{Builder, Env};
    use log::LevelFilter;
    use std::str::FromStr;

    let level = LevelFilter::from_str(log_level_str).unwrap_or(LevelFilter::Info); // Default to Info if parse fails

    Builder::from_env(Env::default().default_filter_or(level.to_string()))
        .filter_module("tungstenite", LevelFilter::Info) // Reduce verbosity from deps
        .filter_module("tokio_tungstenite", LevelFilter::Info)
        .filter_module("hyper", LevelFilter::Info)
        .filter_module("rustls", LevelFilter::Info)
        .try_init()
        .map_err(|e| CoreError::LoggingSetup(e.to_string()))
}

#[cfg(not(feature = "env_logger"))]
pub fn setup_logging(_log_level_str: &str) -> Result<(), CoreError> {
    // No-op if env_logger is not enabled
    log::debug!("env_logger feature not enabled, logging setup skipped via janus-core helper.");
    Ok(())
}
