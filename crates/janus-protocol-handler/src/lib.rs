//! # Janus Protocol Handler
//!
//! Handles the core logic of sending commands, tracking responses,
//! managing request IDs, handling timeouts, and dispatching incoming events.

use actix::prelude::*;
use serde_json::Value; // Re-export Value for convenience

pub mod command_actor;
pub mod event_actor;
pub mod messages;

pub use command_actor::CommandActor;
pub use event_actor::EventActor;
pub use messages::{
    CommandResult,
    ProtocolEvent,
    SendCommand,
    Subscribe,
    Unsubscribe, // Public messages
};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
