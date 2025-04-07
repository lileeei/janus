use std::time::Duration;
use log::{debug, error, info};
use regex::Regex;
use reqwest::Url;
use retry::{delay::Fixed, retry};
use serde::{Deserialize, Serialize};
use tokio::process::Child;

use crate::error::ChromeError;

const MAX_RETRIES: u32 = 5;
const RETRY_DELAY: Duration = Duration::from_millis(100);

#[derive(Debug, Serialize, Deserialize)]
struct DevToolsInfo {
    description: String,
    #[serde(rename = "devtoolsFrontendUrl")]
    devtools_frontend_url: String,
    id: String,
    title: String,
    #[serde(rename = "type")]
    target_type: String,
    url: String,
    #[serde(rename = "webSocketDebuggerUrl")]
    websocket_debugger_url: String,
}

pub struct ChromeLauncher {
    port: u16,
    process: Option<Child>,
}

impl ChromeLauncher {
    pub fn new() -> Result<Self, ChromeError> {
        let port = portpicker::pick_unused_port().ok_or_else(|| {
            ChromeError::LaunchError("Failed to find an available port".to_string())
        })?;

        Ok(Self {
            port,
            process: None,
        })
    }

    pub fn get_debug_ws_url(&self) -> String {
        format!("http://localhost:{}/json/version", self.port)
    }

    pub fn get_port(&self) -> u16 {
        self.port
    }

    pub fn set_process(&mut self, process: Child) {
        self.process = Some(process);
    }

    pub async fn find_ws_url(&self) -> Result<String, ChromeError> {
        let client = reqwest::Client::new();
        let url = self.get_debug_ws_url();

        // 使用重试机制获取 WebSocket URL
        let ws_url = retry(Fixed::new(RETRY_DELAY).take(MAX_RETRIES), || async {
            let response = client.get(&url).send().await.map_err(|e| {
                ChromeError::LaunchError(format!("Failed to connect to Chrome DevTools: {}", e))
            })?;

            if !response.status().is_success() {
                return Err(ChromeError::LaunchError(format!(
                    "Failed to get Chrome DevTools info: HTTP {}",
                    response.status()
                )));
            }

            let info: DevToolsInfo = response.json().await.map_err(|e| {
                ChromeError::LaunchError(format!("Failed to parse Chrome DevTools info: {}", e))
            })?;

            Ok(info.websocket_debugger_url)
        })
        .await?;

        debug!("Found Chrome WebSocket URL: {}", ws_url);
        Ok(ws_url)
    }

    pub fn take_process(&mut self) -> Option<Child> {
        self.process.take()
    }
}

impl Drop for ChromeLauncher {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            let _ = process.start_kill();
        }
    }
} 