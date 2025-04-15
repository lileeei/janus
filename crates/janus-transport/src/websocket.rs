//! Implementation of the `Transport` trait using WebSockets (`tokio-tungstenite`).

#![cfg(feature = "websocket")] // Only compile this module if websocket feature is enabled

use crate::error::TransportError;
use crate::traits::Transport;
use crate::types::{ConnectParams, WebSocketConnectOptions};
use async_trait::async_trait;
use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use log::{debug, error, info, warn};

use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream,
    WebSocketStream,
    connect_async, // Use default connector (can specify later)
    tungstenite::{Error as TungsteniteError, protocol::Message as TungsteniteMessage},
};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type WsSink = SplitSink<WsStream, TungsteniteMessage>;
type WsSource = SplitStream<WsStream>;

/// WebSocket transport implementation.
pub struct WebSocketTransport {
    params: ConnectParams, // Keep params for potential reconnect logic later?
    sink: Option<WsSink>,
    source: Option<WsSource>,
    // Store the raw stream maybe for close? Or rely on Sink/Stream drop?
    // stream: Option<WsStream>,
}

impl WebSocketTransport {
    pub fn new(params: ConnectParams) -> Self {
        Self {
            params,
            sink: None,
            source: None,
            // stream: None,
        }
    }

    fn apply_options(
        _options: &WebSocketConnectOptions,
    ) -> tokio_tungstenite::tungstenite::protocol::WebSocketConfig {
        // Map our WebSocketConnectOptions to tungstenite's WebSocketConfig
        // Example: Allow mapping max_message_size etc.
        let mut config = tokio_tungstenite::tungstenite::protocol::WebSocketConfig::default();
        // if let Some(size) = options.max_message_size {
        //     config.max_message_size = Some(size);
        // }
        // if let Some(size) = options.max_frame_size {
        //     config.max_frame_size = Some(size);
        // }
        // config.accept_unmasked_frames = options.accept_unmasked_frames;
        config // Return the configured options
    }
}

#[async_trait]
impl Transport for WebSocketTransport {
    async fn connect(&mut self) -> Result<(), TransportError> {
        if self.sink.is_some() || self.source.is_some() {
            warn!("WebSocketTransport already connected or partially connected.");
            return Err(TransportError::ConnectionFailed("Already connected".into()));
        }

        info!("Connecting WebSocket to {}", self.params.url);
        let _ws_config = Self::apply_options(&self.params.ws_options);

        // Try connecting to the URL as provided
        info!("Attempting direct connection to {}", self.params.url);
        match connect_async(&self.params.url).await {
            Ok((ws_stream, response)) => {
                debug!("WebSocket handshake successful: {:?}", response);

                let (sink, source) = ws_stream.split();
                self.sink = Some(sink);
                self.source = Some(source);
                // self.stream = Some(ws_stream); // Don't store stream if split

                info!("WebSocket connection established.");
                Ok(())
            }
            Err(err) => {
                // If the URL ends with a slash, try without the trailing slash
                if self.params.url.ends_with('/') {
                    let alt_url = self.params.url.trim_end_matches('/').to_string();
                    info!(
                        "Initial connection failed, trying alternative URL: {}",
                        alt_url
                    );
                    let (ws_stream, response) = connect_async(&alt_url).await?;

                    debug!(
                        "WebSocket handshake successful with alternative URL: {:?}",
                        response
                    );

                    let (sink, source) = ws_stream.split();
                    self.sink = Some(sink);
                    self.source = Some(source);

                    info!("WebSocket connection established with alternative URL.");
                    Ok(())
                } else {
                    // Return the original error
                    Err(err.into())
                }
            }
        }
    }

    async fn disconnect(&mut self) -> Result<(), TransportError> {
        info!("Disconnecting WebSocket.");
        if let Some(mut sink) = self.sink.take() {
            // Attempt to send a Close frame
            match sink.send(TungsteniteMessage::Close(None)).await {
                Ok(_) => debug!("WebSocket Close frame sent."),
                Err(TungsteniteError::ConnectionClosed | TungsteniteError::AlreadyClosed) => {
                    debug!("WebSocket already closed while sending Close frame.")
                }
                Err(e) => {
                    warn!(
                        "Error sending WebSocket Close frame: {}. Closing anyway.",
                        e
                    );
                    // Map error? sink.close() will likely also fail.
                }
            }
            // Close the sink explicitly
            if let Err(e) = sink.close().await {
                // Ignore AlreadyClosed errors as they are expected if read side closed first
                if !matches!(
                    e,
                    TungsteniteError::ConnectionClosed | TungsteniteError::AlreadyClosed
                ) {
                    warn!("Error closing WebSocket sink: {}", e);
                    // Still proceed to drop source etc.
                }
            }
        } else {
            warn!("WebSocket sink already taken or never existed during disconnect.");
        }

        // Drop the source stream
        self.source = None;
        // self.stream = None; // Clear stream if stored separately

        info!("WebSocket disconnected.");
        Ok(())
    }

    async fn send(&mut self, message: &str) -> Result<(), TransportError> {
        let sink = self
            .sink
            .as_mut()
            .ok_or_else(|| TransportError::NotConnected("WebSocket sink unavailable".into()))?;

        debug!("Sending WebSocket message: {}", message); // May be too verbose for production
        sink.send(TungsteniteMessage::Text(message.to_string()))
            .await?;
        Ok(())
    }

    async fn receive(&mut self) -> Option<Result<String, TransportError>> {
        let source = self.source.as_mut()?; // Returns None if source is None

        match source.next().await {
            Some(Ok(msg)) => {
                match msg {
                    TungsteniteMessage::Text(text) => {
                        debug!("Received WebSocket Text: {}", text); // May be too verbose
                        Some(Ok(text))
                    }
                    TungsteniteMessage::Binary(bin) => {
                        warn!(
                            "Received unexpected WebSocket Binary message ({} bytes), ignoring.",
                            bin.len()
                        );
                        // Skip binary messages and get the next one
                        self.receive().await
                    }
                    TungsteniteMessage::Ping(data) => {
                        debug!("Received WebSocket Ping: {:?}", data);
                        // Tungstenite Sink should handle responding to Pings automatically
                        // If not, we'd need to send a Pong here.
                        // Let's continue waiting for the next message. Loop in caller.
                        // Need to recurse or loop here to continue waiting.
                        // TODO: Re-evaluate how to handle Ping/Pong transparently.
                        // For now, treat it as non-data and wait for next frame.
                        self.receive().await; // Recursive call - Careful with stack depth! Loop preferred.
                        // Let's simplify and return an error for now, or ignore.
                        // Returning None might prematurely end the read loop.
                        // Best: Let the ConnectionActor loop handle this.
                        // For simplicity here: return error or skip. Let's return error.
                        // Some(Err(TransportError::Other("Received control frame (Ping)".into())))
                        // Let's try ignoring and continuing the wait:
                        // self.receive().await // CAUTION: Potential stack overflow
                        // ** Safer Approach: Return a special marker? Or let caller loop. **
                        // Simplest for now: Indicate non-data received
                        self.receive().await // Call receive again to get the next message
                    }
                    TungsteniteMessage::Pong(data) => {
                        debug!("Received WebSocket Pong: {:?}", data);
                        // Ignore Pongs and get the next message
                        self.receive().await // Call receive again
                    }
                    TungsteniteMessage::Close(close_frame) => {
                        info!("Received WebSocket Close frame: {:?}", close_frame);
                        None // Signal graceful closure
                    }
                    TungsteniteMessage::Frame(_) => {
                        // Raw frame, likely shouldn't happen with high-level functions
                        warn!("Received unexpected WebSocket raw frame, ignoring.");
                        Some(Err(TransportError::ReceiveFailed(
                            "Received unexpected raw frame".into(),
                        )))
                    }
                }
            }
            Some(Err(e)) => {
                // Handle different Tungstenite errors
                match e {
                    TungsteniteError::ConnectionClosed | TungsteniteError::AlreadyClosed => {
                        info!("WebSocket connection closed while receiving.");
                        None // Treat as graceful close if error indicates closure
                    }
                    TungsteniteError::Io(_) | TungsteniteError::Tls(_) => {
                        error!("WebSocket IO/TLS error during receive: {}", e);
                        Some(Err(e.into())) // Convert to TransportError
                    }
                    TungsteniteError::Utf8 => {
                        error!("Received invalid UTF-8 data: {}", e);
                        Some(Err(TransportError::ReceiveFailed("Invalid UTF-8".into())))
                    }
                    // Treat protocol errors, capacity errors etc. as fatal receive errors
                    _ => {
                        error!("WebSocket receive error: {}", e);
                        Some(Err(e.into()))
                    }
                }
            }
            None => {
                info!("WebSocket stream ended (source returned None).");
                None // Stream naturally ended
            }
        }
    }
}

// Ensure Transport is Unpin, WebSocketTransport should be if fields are.
// Box<dyn Transport> requires Unpin. Sink/Source from futures_util are Unpin.
impl Unpin for WebSocketTransport {}
