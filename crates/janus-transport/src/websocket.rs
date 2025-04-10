//! Implementation of the `Transport` trait using WebSockets (`tokio-tungstenite`).

#![cfg(feature = "websocket")] // Only compile this module if websocket feature is enabled

use crate::error::TransportError;
use crate::traits::Transport;
use crate::types::{ConnectParams, WebSocketConnectOptions};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt, stream::{SplitSink, SplitStream}};
use log::{debug, error, info, warn};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async, // Use default connector (can specify later)
    tungstenite::{protocol::Message as TungsteniteMessage, Error as TungsteniteError},
    MaybeTlsStream, WebSocketStream,
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
            return Err(TransportError::ConnectionFailed(
                "Already connected".into(),
            ));
        }

        info!("Connecting WebSocket to {}", self.params.url);
        let ws_config = Self::apply_options(&self.params.ws_options);

        // connect_async_with_config might be needed for options
        let (ws_stream, response) = connect_async(&self.params.url).await?;

        debug!("WebSocket handshake successful: {:?}", response);

        let (sink, source) = ws_stream.split();
        self.sink = Some(sink);
        self.source = Some(source);
        // self.stream = Some(ws_stream); // Don't store stream if split

        info!("WebSocket connection established.");
        Ok(())
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
                    warn!("Error sending WebSocket Close frame: {}. Closing anyway.", e);
                    // Map error? sink.close() will likely also fail.
                }
            }
             // Close the sink explicitly
            if let Err(e) = sink.close().await {
                // Ignore AlreadyClosed errors as they are expected if read side closed first
                 if !matches!(e, TungsteniteError::ConnectionClosed | TungsteniteError::AlreadyClosed) {
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
        let sink = self.sink.as_mut().ok_or_else(|| {
            TransportError::NotConnected("WebSocket sink unavailable".into())
        })?;

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
                    },
                    TungsteniteMessage::Binary(bin) => {
                        warn!("Received unexpected WebSocket Binary message ({} bytes), ignoring.", bin.len());
                        // Recursively call receive again to wait for the next message?
                        // Or return an error? For now, let's just loop implicitly in the caller.
                        // This might need a loop internally to skip non-text messages.
                         Some(Err(TransportError::ReceiveFailed("Received unexpected binary message".into())))
                    }
                    TungsteniteMessage::Ping(data) => {
                        debug!("Received WebSocket Ping: {:?}", data);
                        // Tungstenite Sink should handle responding to Pings automatically
                        // If not, we'd need to send a Pong here.
                        // Let's continue waiting for the next message. Loop in caller.
                        // Need to recurse or loop here to continue waiting.
                        // TODO: Re-evaluate how to handle Ping/Pong transparently.
                        // For now, treat it as non-data and wait for next frame.
                         self.receive().await // Recursive call - Careful with stack depth! Loop preferred.
                                             // Let's simplify and return an error for now, or ignore.
                                             // Returning None might prematurely end the read loop.
                                             // Best: Let the ConnectionActor loop handle this.
                                             // For simplicity here: return error or skip. Let's return error.
                         // Some(Err(TransportError::Other("Received control frame (Ping)".into())))
                         // Let's try ignoring and continuing the wait:
                         // self.receive().await // CAUTION: Potential stack overflow
                         // ** Safer Approach: Return a special marker? Or let caller loop. **
                         // Simplest for now: Indicate non-data received
                         Some(Err(TransportError::Other("Received Ping".into()))) // Caller should retry
                    }
                    TungsteniteMessage::Pong(data) => {
                        debug!("Received WebSocket Pong: {:?}", data);
                        // Ignore Pongs, continue waiting. Similar issue as Ping.
                        Some(Err(TransportError::Other("Received Pong".into()))) // Caller should retry
                    }
                    TungsteniteMessage::Close(close_frame) => {
                        info!("Received WebSocket Close frame: {:?}", close_frame);
                        None // Signal graceful closure
                    }
                    TungsteniteMessage::Frame(_) => {
                         // Raw frame, likely shouldn't happen with high-level functions
                         warn!("Received unexpected WebSocket raw frame, ignoring.");
                         Some(Err(TransportError::ReceiveFailed("Received unexpected raw frame".into())))
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
