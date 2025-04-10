use crate::error::CoreError;
use config::{Config as ConfigLoader, Environment, File, Source};
use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf, time::Duration};

// Helper for deserializing Duration from milliseconds
mod duration_ms_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_millis() as u64)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

// Main configuration structure
#[derive(Deserialize, Debug, Clone, Default)]
#[serde(default)] // Ensure fields default if missing in config source
pub struct Config {
    pub global: GlobalConfig,
    pub transport: TransportConfig,
    pub actor_system: ActorSystemConfig,
    pub browser_defaults: BrowserLaunchConfig, // Default launch settings
    // Use BTreeMap for consistent ordering if serialized/logged
    pub browsers: HashMap<String, BrowserLaunchConfig>, // Browser-specific overrides
}

// Global settings
#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct GlobalConfig {
    pub log_level: String,
    #[serde(with = "duration_ms_serde")]
    pub default_command_timeout: Duration,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            default_command_timeout: Duration::from_secs(30),
        }
    }
}

// Transport layer configuration
#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct TransportConfig {
    #[serde(with = "duration_ms_serde")]
    pub connect_timeout: Duration,
    #[cfg(feature = "websocket")]
    pub websocket: WebSocketConfig,
    // pub tcp: Option<TcpConfig>, // Add later if needed
    // pub ipc: Option<IpcConfig>, // Add later if needed
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(20),
            #[cfg(feature = "websocket")]
            websocket: Default::default(),
        }
    }
}

// WebSocket specific configuration
#[derive(Deserialize, Debug, Clone, Default)]
#[cfg(feature = "websocket")]
#[serde(default)]
pub struct WebSocketConfig {
    pub max_message_size: Option<usize>,
    pub max_frame_size: Option<usize>,
    // Default to false, as it's less common
    pub accept_unmasked_frames: bool,
    // Add TLS options later if needed (e.g., custom certs)
}

// Actor system tuning parameters
#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct ActorSystemConfig {
    pub default_mailbox_capacity: usize,
}

impl Default for ActorSystemConfig {
    fn default() -> Self {
        Self {
            default_mailbox_capacity: 100,
        }
    }
}

// Configuration for launching and connecting to a browser instance
#[derive(Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct BrowserLaunchConfig {
    // Launch options
    pub executable_path: Option<PathBuf>,
    pub user_data_dir: Option<PathBuf>,
    pub headless: Option<bool>,
    pub args: Option<Vec<String>>,
    pub env_vars: Option<HashMap<String, String>>,

    // Connection options
    pub remote_debugging_address: Option<String>, // Just IP or hostname
    pub remote_debugging_port: Option<u16>,
    pub connection_url_override: Option<String>, // Full WS/TCP URL
    pub protocol: Option<BrowserProtocol>,       // CDP, BiDi

    // Protocol-specific settings
    pub cdp_settings: Option<CdpSettings>,
    pub bidi_settings: Option<BidiSettings>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")] // Allows config values "cdp", "bidi"
pub enum BrowserProtocol {
    Cdp,
    BiDi, // Consider WebDriverBiDi for clarity if needed elsewhere
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct CdpSettings {
    pub use_flattened_target_info: bool,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct BidiSettings {
    // Use serde_json::Value for flexibility
    pub capabilities: Option<serde_json::Value>,
}

// Merging logic remains the same as in the design doc
impl BrowserLaunchConfig {
    pub fn merged_with(&self, defaults: &BrowserLaunchConfig) -> Self {
        Self {
            executable_path: self
                .executable_path
                .clone()
                .or_else(|| defaults.executable_path.clone()),
            user_data_dir: self
                .user_data_dir
                .clone()
                .or_else(|| defaults.user_data_dir.clone()),
            headless: self.headless.or(defaults.headless),
            args: self.args.clone().or_else(|| defaults.args.clone()),
            env_vars: self.env_vars.clone().or_else(|| defaults.env_vars.clone()),
            remote_debugging_address: self
                .remote_debugging_address
                .clone()
                .or_else(|| defaults.remote_debugging_address.clone()),
            remote_debugging_port: self
                .remote_debugging_port
                .or(defaults.remote_debugging_port),
            connection_url_override: self
                .connection_url_override
                .clone()
                .or_else(|| defaults.connection_url_override.clone()),
            protocol: self.protocol.clone().or_else(|| defaults.protocol.clone()),
            cdp_settings: self
                .cdp_settings
                .clone()
                .or_else(|| defaults.cdp_settings.clone()),
            bidi_settings: self
                .bidi_settings
                .clone()
                .or_else(|| defaults.bidi_settings.clone()),
        }
    }
}

/// Loads configuration from default locations and environment variables.
///
/// Looks for `janus.toml` (or `.json`, `.yaml`, etc.) in the current directory.
/// Overrides with environment variables prefixed with `JANUS_`.
/// (e.g., `JANUS_GLOBAL__LOG_LEVEL=debug`, `JANUS_TRANSPORT__CONNECT_TIMEOUT_MS=10000`)
/// Note the double underscore `__` for nested fields when using `Environment`.
pub fn load_config() -> Result<Config, CoreError> {
    let builder = ConfigLoader::builder()
        // Add default values (though struct defaults handle most cases)
        // Set defaults using the structure helps if some fields aren't Option<>
        .set_default("global.log_level", "info")?
        .set_default("global.default_command_timeout_ms", 30000u64)?
        .set_default("transport.connect_timeout_ms", 20000u64)?
        .set_default("actor_system.default_mailbox_capacity", 100usize)?
        // Add other non-Option defaults if any exist
        // Load from `janus.toml` (or other supported extensions) if it exists
        .add_source(File::with_name("janus").required(false))
        // Load from environment variables (e.g., JANUS_GLOBAL__LOG_LEVEL)
        // Use "__" as separator for nested structures
        .add_source(
            Environment::with_prefix("JANUS")
                .separator("__")
                .try_parsing(true),
        )
        .build()?;

    // Deserialize the loaded configuration into the Config struct
    builder.try_deserialize().map_err(CoreError::ConfigLoad)
}
