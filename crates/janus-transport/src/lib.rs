//! # Janus Transport (L3 - Raw Communication)
//!
//! This crate handles the low-level details of establishing and managing
//! network connections (like WebSockets) to browser debugging endpoints.
//!
//! It defines the `Transport` trait for abstracting different communication
//! methods and provides the `ConnectionActor` for managing the lifecycle
//! and message flow over a single connection within the actor system.

pub mod connection;
pub mod error;
pub mod factory;
pub mod traits;
pub mod types;
#[cfg(feature = "websocket")]
pub mod websocket; // Added factory module

// Re-export key items
pub use connection::{
    ConnectionActor, ConnectionState, ConnectionStatusUpdate, IncomingMessage, SendMessage,
};
pub use error::TransportError;
pub use factory::create_transport;
pub use traits::Transport;
pub use types::{ConnectParams, WebSocketConnectOptions};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
