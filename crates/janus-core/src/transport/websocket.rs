use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt, Stream};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio_tungstenite::{
    connect_async,
    tungstenite::protocol::{Message as WsMessage, CloseFrame},
};
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use crate::error::TransportError;
use super::{Transport, ConnectParams};

/// WebSocket-based transport implementation
pub struct WebSocketTransport {
    stream: tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
}

impl Stream for WebSocketTransport {
    type Item = Result<String, TransportError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        match this.stream.poll_next_unpin(cx) {
            Poll::Ready(Some(Ok(msg))) => match msg {
                WsMessage::Text(text) => Poll::Ready(Some(Ok(text))),
                WsMessage::Close(frame) => {
                    let reason = frame
                        .map(|f| format!("code: {}, reason: {}", f.code, f.reason))
                        .unwrap_or_else(|| "no reason given".to_string());
                    Poll::Ready(Some(Err(TransportError::ConnectionClosed { reason })))
                }
                _ => Poll::Ready(None), // Ignore other message types
            },
            Poll::Ready(Some(Err(e))) => {
                Poll::Ready(Some(Err(TransportError::WebSocket(e.to_string()))))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl WebSocketTransport {
    pub async fn new(url: String) -> Result<Self, TransportError> {
        let (ws_stream, _) = connect_async(&url)
            .await
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

        Ok(Self { stream: ws_stream })
    }
}

#[async_trait]
impl Transport for WebSocketTransport {
    type Sink = Pin<Box<dyn futures_util::Sink<String, Error = TransportError> + Send + Unpin>>;

    async fn connect(params: ConnectParams) -> Result<(Self, Self::Sink), TransportError> {
        let transport = Self::new(params.url).await?;
        let sink = Box::pin(transport.stream.with(|s: String| async move {
            Ok(tokio_tungstenite::tungstenite::Message::Text(s))
        }));
        Ok((transport, sink))
    }

    async fn disconnect(&mut self) -> Result<(), TransportError> {
        let close_frame = CloseFrame {
            code: CloseCode::Normal,
            reason: "Client disconnecting".into(),
        };
        self.stream.close(Some(close_frame))
            .await
            .map_err(|e| TransportError::DisconnectFailed(e.to_string()))
    }
}

impl futures_util::sink::Sink<String> for WebSocketTransport {
    type Error = TransportError;

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let this = self.get_mut();
        this.stream
            .poll_ready_unpin(cx)
            .map_err(|e| TransportError::WebSocket(e.to_string()))
    }

    fn start_send(
        self: std::pin::Pin<&mut Self>,
        item: String,
    ) -> Result<(), Self::Error> {
        let this = self.get_mut();
        this.stream
            .start_send_unpin(WsMessage::Text(item))
            .map_err(|e| TransportError::WebSocket(e.to_string()))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let this = self.get_mut();
        this.stream
            .poll_flush_unpin(cx)
            .map_err(|e| TransportError::WebSocket(e.to_string()))
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let this = self.get_mut();
        this.stream
            .poll_close_unpin(cx)
            .map_err(|e| TransportError::WebSocket(e.to_string()))
    }
} 