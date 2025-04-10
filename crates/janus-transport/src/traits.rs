use crate::error::TransportError;
use async_trait::async_trait;

/// Represents an abstract transport mechanism for sending and receiving
/// string-based messages (typically JSON) over a network connection.
///
/// Implementations handle the specifics of protocols like WebSockets or TCP.
#[async_trait]
pub trait Transport: Send + Unpin {
    /// Establishes the connection based on parameters provided during creation.
    async fn connect(&mut self) -> Result<(), TransportError>;

    /// Closes the connection gracefully.
    async fn disconnect(&mut self) -> Result<(), TransportError>;

    /// Sends a message over the established connection.
    ///
    /// # Arguments
    /// * `message` - The string message to send. Borrowed to potentially avoid clones.
    async fn send(&mut self, message: &str) -> Result<(), TransportError>;

    /// Waits for and returns the next message received from the connection.
    ///
    /// # Returns
    /// * `Some(Ok(String))` - Successfully received a message.
    /// * `Some(Err(TransportError))` - An error occurred while receiving.
    /// * `None` - The connection was closed gracefully from the remote end.
    async fn receive(&mut self) -> Option<Result<String, TransportError>>;
}
