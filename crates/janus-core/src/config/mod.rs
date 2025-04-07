use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use crate::error::{CoreError, ConfigError}; // Use CoreError for config loading result

// Re-export Config for easier access
pub use config::ConfigError;

// --- Struct Definitions (copied from design) ---

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default)]
    pub global: GlobalConfig,
    #[serde(default)]
    pub browser_defaults: BrowserDefaults,
    #[serde(default)]
    pub browsers: HashMap<String, BrowserSpecificConfig>,
    #[serde(default)]
    pub transport: TransportConfig,
    #[serde(default)]
    pub actor_system: ActorSystemConfig,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct GlobalConfig {
    pub log_level: String,
    pub default_command_timeout_ms: u64,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            default_command_timeout_ms: 30_000, // 30 seconds
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct BrowserDefaults {
    pub user_data_dir_base: Option<String>,
    pub headless: bool,
    pub args: Vec<String>,
}

impl Default for BrowserDefaults {
     fn default() -> Self {
        Self {
            user_data_dir_base: None,
            headless: true,
            args: vec![],
        }
    }
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct BrowserSpecificConfig {
    pub executable_path: Option<String>,
    pub user_data_dir: Option<String>,
    pub args: Option<Vec<String>>,
    pub protocol_port: Option<u16>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct TransportConfig {
    pub connect_timeout_ms: u64,
    pub websocket: WebSocketConfig,
}

 impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            connect_timeout_ms: 10_000, // 10 seconds
            websocket: WebSocketConfig::default(),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct WebSocketConfig {
    pub max_message_size: Option<usize>,
    pub accept_unmasked_frames: bool,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            max_message_size: Some(64 * 1024 * 1024), // 64 MiB
            accept_unmasked_frames: false,
        }
    }
}

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


// --- Loading Logic ---

pub fn load_config(source_path: Option<PathBuf>) -> Result<Config, CoreError> {
    let default_config_name = "janus_config"; // Base name for config files

    let mut builder = config::Config::builder()
        // Add default values baked into the structs via #[serde(default)]
        // Note: Default values from `impl Default` combined with `#[serde(default)]`
        // mean we usually don't need explicit `.set_default()` unless overriding those base defaults.
        // However, keeping them can make defaults more explicit if desired.
        .set_default("global.log_level", GlobalConfig::default().log_level).map_err(CoreError::Config)?
        .set_default("transport.connect_timeout_ms", TransportConfig::default().connect_timeout_ms).map_err(CoreError::Config)?
        .set_default("actor_system.default_mailbox_capacity", ActorSystemConfig::default().default_mailbox_capacity).map_err(CoreError::Config)?;


    // Load from specified file path if provided
    if let Some(path) = source_path {
        if path.exists() {
             log::debug!("Loading configuration from: {:?}", path);
            builder = builder.add_source(config::File::from(path).required(true));
        } else {
             log::warn!("Specified configuration file not found: {:?}", path);
             // Consider returning an error if the specified file is mandatory
             // return Err(CoreError::Config(ConfigError::NotFound(path.to_string_lossy().to_string())));
        }
    } else {
         // Load from default locations if no specific path is given
         // e.g., ./janus_config.toml, ~/.config/janus/config.toml etc.
         log::debug!("Attempting to load configuration from default locations (e.g., {}.toml)", default_config_name);
         builder = builder.add_source(
             config::File::with_name(default_config_name).required(false) // Optional load
         );
         // TODO: Add user config dir path lookup if desired
         // (e.g., using dirs_next crate)
    }

    // Load from environment variables (e.g., JANUS_GLOBAL_LOG_LEVEL)
    builder = builder.add_source(
        config::Environment::with_prefix("JANUS")
            .separator("_") // e.g., JANUS_TRANSPORT_CONNECT_TIMEOUT_MS
            .try_parsing(true) // Attempt to parse bools, ints etc.
            .list_separator(",") // For Vec<String> like args
            .with_list_parse_key("global.args") // Specify keys that should be parsed as lists
            .with_list_parse_key("browser_defaults.args")
            .with_list_parse_key("browsers.*.args") // Need careful handling for nested map keys like this in config crate <= 0.14
    );

    // Build and deserialize
    let cfg = builder.build().map_err(CoreError::Config)?
              .try_deserialize::<Config>().map_err(CoreError::Config)?;

    log::debug!("Successfully loaded configuration: {:?}", cfg);
    Ok(cfg)
}
