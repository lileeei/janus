use actix::io::{ FramedWrite };
use actix::prelude::*;
use async_trait::async_trait;
use futures_util::stream::StreamExt;
use std::time::Duration;
use tokio_util::codec::Encoder;
use bytes::BytesMut;
use crate::{TransportError, ProtocolError};
use super::*;
use std::pin::Pin;
use actix::{Actor, Context, Handler, Message, StreamHandler};
use futures_util::{Sink, SinkExt, Stream, StreamExt};
use tokio_util::codec::{Decoder};
use janus_interface::transport::*;

// Use a specific ConnectionId type alias from janus-core or define locally
pub type ConnectionId = u64;

#[derive(Debug, Clone)]
pub struct ConnectParams {
    pub url: String,
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
    pub ws_config: Option<tokio_tungstenite::tungstenite::protocol::WebSocketConfig>,
}

pub trait Transport: Send + Unpin + 'static {
    type Sink: Sink<String, Error = TransportError> + Send + Unpin + 'static;
    
    async fn connect(params: ConnectParams) -> Result<(Self, Self::Sink), TransportError> 
    where
        Self: Sized;
        
    async fn disconnect(&mut self) -> Result<(), TransportError>;
}

// --- Connection Actor ---

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Idle,
    Connecting,
    Connected,
    Disconnecting,
    Disconnected(Option<TransportError>),
}

/// Public message to report connection status changes (sent to supervisor).
#[derive(Message, Debug, Clone)]
#[rtype(result = "()")]
pub struct ConnectionStatusUpdate {
    pub id: ConnectionId,
    pub state: ConnectionState,
}

/// Actor responsible for managing a single underlying transport connection.
pub struct ConnectionActor<T: Transport> {
    transport: Option<T>,
    sink: Option<T::Sink>,
    state: ConnectionState,
}

impl<T: Transport> ConnectionActor<T> {
    pub fn new() -> Self {
        Self {
            transport: None,
            sink: None,
            state: ConnectionState::Disconnected,
        }
    }
}

impl<T: Transport> Actor for ConnectionActor<T> {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        log::info!("Connection actor started");
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        log::info!("Connection actor stopped");
    }
}

impl<T: Transport> Handler<ConnectionEstablished<T>> for ConnectionActor<T> {
    type Result = ();

    fn handle(&mut self, msg: ConnectionEstablished<T>, _ctx: &mut Context<Self>) {
        self.transport = Some(msg.transport);
        self.sink = Some(msg.sink);
        self.state = ConnectionState::Connected;
        log::info!("Connection established");
    }
}

impl<T: Transport> Handler<SendRawMessage> for ConnectionActor<T> {
    type Result = ();

    fn handle(&mut self, msg: SendRawMessage, ctx: &mut Context<Self>) {
        if let Some(sink) = self.sink.as_mut() {
            let fut = sink.send(msg.0);
            let actor = ctx.address();
            
            actix::spawn(async move {
                if let Err(e) = fut.await {
                    log::error!("Failed to send message: {}", e);
                    actor.do_send(ConnectionStatusUpdate::Error(e));
                }
            });
        }
    }
}

// --- Codec for FramedWrite ---

#[derive(Default)]
pub struct ConnectionCodec;

impl Decoder for ConnectionCodec {
    type Item = String;
    type Error = TransportError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }

        let data = String::from_utf8(src.split().to_vec())
            .map_err(|e| TransportError::InvalidData(e.to_string()))?;
        Ok(Some(data))
    }
}

impl Encoder<String> for ConnectionCodec {
    type Error = TransportError;

    fn encode(&mut self, item: String, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.extend_from_slice(item.as_bytes());
        Ok(())
    }
}

// --- Helper trait for FramedWrite ---
pub trait ActorFrame: futures_util::sink::Sink<String, Error = TransportError> + Unpin + 'static {}
