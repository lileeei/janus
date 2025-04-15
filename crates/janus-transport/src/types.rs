use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Parameters required to establish a connection.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConnectParams {
    /// The full URL for the connection (e.g., "ws://127.0.0.1:9222/devtools/browser/...").
    /// The scheme determines the transport type (ws, wss, tcp, etc.).
    pub url: String,

    /// Connection timeout. Applied during the initial connection attempt.
    #[serde(with = "serde_duration_ms", default = "default_connect_timeout")]
    pub connection_timeout: Duration,

    /// Options specific to WebSocket connections.
    #[cfg(feature = "websocket")]
    #[serde(default)]
    pub ws_options: WebSocketConnectOptions,
    // Add other transport-specific options here as needed
    // pub tcp_options: Option<TcpConnectOptions>,
}

fn default_connect_timeout() -> Duration {
    Duration::from_secs(20)
}

/// Options specific to WebSocket connections.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[cfg(feature = "websocket")]
#[serde(default)]
pub struct WebSocketConnectOptions {
    pub max_message_size: Option<usize>,
    pub max_frame_size: Option<usize>,
    pub accept_unmasked_frames: bool,
    // Add headers, protocols, compression options etc. later if needed
    // pub custom_headers: Option<HashMap<String, String>>,
}

// Module for serializing/deserializing Duration to/from milliseconds
pub(crate) mod serde_duration_ms {
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
