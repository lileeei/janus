use actix::prelude::*;
use async_trait::async_trait;
use futures_util::stream::StreamExt;
use std::time::Duration;
use crate::{TransportError, ApiError};

/// Unique identifier for a connection
pub type ConnectionId = u64;

/// Parameters for establishing a connection
#[derive(Debug, Clone)]
pub struct ConnectParams {
    pub url: String,
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
    pub ws_config: Option<tokio_tungstenite::tungstenite::protocol::WebSocketConfig>,
}

/// State of a connection
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Idle,
    Connecting,
    Connected,
    Disconnecting,
    Disconnected(Option<TransportError>),
}

/// Message sent to supervisor when connection status changes
#[derive(Message, Debug, Clone)]
#[rtype(result = "()")]
pub struct ConnectionStatusUpdate {
    pub id: ConnectionId,
    pub state: ConnectionState,
}

/// Message for sending raw data through the transport
#[derive(Message)]
#[rtype(result = "Result<(), TransportError>")]
pub struct SendRawMessage(pub String);

/// Message for receiving raw data from the transport
#[derive(Message)]
#[rtype(result = "()")]
pub struct IncomingRawMessage(pub String);

/// Transport trait for implementing different transport protocols
#[async_trait]
pub trait Transport: Send + Unpin + StreamExt<Item = Result<String, TransportError>> + 'static {
    /// Type for the write half of the transport
    type Sink: futures_util::sink::Sink<String, Error = TransportError> + Send + Unpin + 'static;

    /// Connect to the transport
    async fn connect(params: ConnectParams) -> Result<(Self, Self::Sink), TransportError> where Self: Sized;

    /// Disconnect from the transport
    async fn disconnect(sink: Self::Sink) -> Result<(), TransportError>;
}

pub mod connection;
pub mod websocket;

pub use connection::ConnectionActor;
pub use websocket::WebSocketTransport;

/// Creates and starts the appropriate ConnectionActor based on the URL scheme.
pub fn create_transport_actor(
    id: ConnectionId,
    params: ConnectParams,
    message_handler: Recipient<IncomingRawMessage>,
    supervisor: Option<Recipient<ConnectionStatusUpdate>>,
) -> Result<Addr<ConnectionActor<WebSocketTransport>>, TransportError> {
    let url_scheme = url::Url::parse(&params.url)
        .map_err(|e| TransportError::InvalidUrl(e.to_string()))?
        .scheme()
        .to_lowercase();

    match url_scheme.as_str() {
        "ws" | "wss" => {
            let actor = ConnectionActor::<WebSocketTransport>::new(
                id,
                params,
                message_handler,
                supervisor,
            );
            let addr = actor.start();
            Ok(addr)
        }
        _ => Err(TransportError::UnsupportedScheme(url_scheme)),
    }
} 