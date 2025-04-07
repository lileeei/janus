use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use janus_core::actor::ActorConfig;
use janus_core::error::ActorError;

#[derive(Debug, Clone)]
pub struct ChromeBrowserConfig {
    pub executable_path: Option<String>,
    pub user_data_dir: Option<PathBuf>,
    pub headless: bool,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub default_viewport: Option<Viewport>,
    pub ignore_https_errors: bool,
    pub default_timeout: Duration,
    pub max_concurrent_pages: usize,
}

#[derive(Debug, Clone)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
    pub device_scale_factor: f64,
    pub is_mobile: bool,
    pub has_touch: bool,
    pub is_landscape: bool,
}

impl Default for ChromeBrowserConfig {
    fn default() -> Self {
        Self {
            executable_path: None,
            user_data_dir: None,
            headless: true,
            args: vec![],
            env: HashMap::new(),
            default_viewport: Some(Viewport {
                width: 1280,
                height: 720,
                device_scale_factor: 1.0,
                is_mobile: false,
                has_touch: false,
                is_landscape: true,
            }),
            ignore_https_errors: false,
            default_timeout: Duration::from_secs(30),
            max_concurrent_pages: 10,
        }
    }
}

impl ActorConfig for ChromeBrowserConfig {
    fn validate(&self) -> Result<(), ActorError> {
        if let Some(ref path) = self.executable_path {
            if !std::path::Path::new(path).exists() {
                return Err(ActorError::InitializationError(
                    format!("Chrome executable not found at: {}", path)
                ));
            }
        }
        if self.max_concurrent_pages == 0 {
            return Err(ActorError::InitializationError(
                "max_concurrent_pages must be greater than 0".to_string()
            ));
        }
        Ok(())
    }
} 