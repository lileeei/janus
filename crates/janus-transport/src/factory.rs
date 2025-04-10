//! Factory function for creating Transport implementations based on ConnectParams.

use crate::error::TransportError;
use crate::traits::Transport;
use crate::types::ConnectParams;

#[cfg(feature = "websocket")]
use crate::websocket::WebSocketTransport;

/// Creates a boxed `Transport` trait object based on the URL scheme in `ConnectParams`.
///
/// Currently supports `ws://` and `wss://` if the `websocket` feature is enabled.
pub fn create_transport(params: &ConnectParams) -> Result<Box<dyn Transport>, TransportError> {
    let url = &params.url;
    log::debug!("Attempting to create transport for URL: {}", url);

    if url.starts_with("ws://") || url.starts_with("wss://") {
        #[cfg(feature = "websocket")]
        {
            log::info!("Creating WebSocketTransport for {}", url);
            Ok(Box::new(WebSocketTransport::new(params.clone())))
        }
        #[cfg(not(feature = "websocket"))]
        {
            log::error!("WebSocket URL specified, but 'websocket' feature is not enabled.");
            Err(TransportError::UnsupportedScheme(
                "WebSocket (ws/wss) requires the 'websocket' feature.".to_string(),
            ))
        }
    }
    // --- Add other schemes later ---
    // else if url.starts_with("tcp://") {
    //     #[cfg(feature = "tcp")]
    //     {
    //         // Ok(Box::new(TcpTransport::new(params.clone())))
    //         Err(TransportError::UnsupportedScheme("TCP transport not yet implemented".to_string()))
    //     }
    //     #[cfg(not(feature = "tcp"))]
    //     {
    //          Err(TransportError::UnsupportedScheme(
    //             "TCP requires the 'tcp' feature.".to_string(),
    //         ))
    //     }
    // }
    else {
        log::error!("Unsupported URL scheme found in: {}", url);
        Err(TransportError::UnsupportedScheme(format!(
            "Scheme not supported or feature not enabled for URL: {}",
            url
        )))
    }
}
