use actix::prelude::*;
// Use correct error types in message Results and Handlers
use crate::error::{CoreError, TransportError, ProtocolError, MailboxError};
use crate::config; // Import config if needed by SupervisorActor
// Import necessary types from janus-transport
// Adjust imports to use the new function signature and potentially specific actor type
use janus_transport::{ConnectParams, ConnectionState, ConnectionStatusUpdate, create_transport_actor};
use janus_transport::WebSocketTransport; // Assuming WebSocket is primary for now
use janus_transport::ConnectionActor; // Import the concrete actor type if needed for Addr type
use std::collections::HashMap;
use url::Url; // Use the url crate
use janus_interface::transport::*;
use janus_interface::{TransportError, ProtocolError};

// Re-export message types
pub use janus_interface::transport::{SendRawMessage, IncomingRawMessage};

// --- Common Actor Messages ---

/// Message to send a raw string payload over the connection.
/// Handled by ConnectionActor.
#[derive(Message, Debug, Clone)]
#[rtype(result = "Result<(), TransportError>")] // Use TransportError
pub struct SendRawMessage(pub String);

/// Message representing a raw string payload received from the connection.
/// Sent *by* ConnectionActor to a designated handler (e.g., Command/Event Actor).
#[derive(Message, Debug, Clone)]
#[rtype(result = "()")]
pub struct IncomingRawMessage(pub String);

/// Internal representation of a command to be executed.
/// Sent *to* CommandActor.
#[derive(Message, Debug)]
#[rtype(result = "Result<serde_json::Value, ProtocolError>")] // Use ProtocolError
pub struct ExecuteCommand {
    pub target_id: Option<String>, // Or SessionId, context dependent
    pub method: String,
    pub params: serde_json::Value,
    // reply_to is implicit in actix request/response
}

/// Internal representation of a browser event.
/// Sent *to* EventActor or distributed *by* EventActor.
#[derive(Message, Debug, Clone)]
#[rtype(result = "()")]
pub struct ProtocolEvent {
     pub session_id: Option<String>, // If applicable (e.g., CDP sessions)
     pub method: String, // The event name (e.g., "Page.loadEventFired")
     pub params: serde_json::Value,
}

// --- Placeholder Core Actors ---
// Define them here or in separate modules (e.g., core/actor/command.rs)

#[derive(Debug)]
pub struct CommandActor;
impl Actor for CommandActor { type Context = Context<Self>; }
// Basic handler for IncomingRawMessage (replace with actual logic later)
impl Handler<IncomingRawMessage> for CommandActor {
    type Result = ();
    fn handle(&mut self, msg: IncomingRawMessage, _ctx: &mut Context<Self>) {
        log::debug!("Placeholder Command Actor received raw message: {}...", msg.0.chars().take(100).collect::<String>());
        // In reality, this actor would parse the message, check if it's a response (has ID),
        // find the pending command, and send the result back to the requester.
    }
}

#[derive(Debug)]
pub struct EventActor;
impl Actor for EventActor { type Context = Context<Self>; }
// Basic handler for IncomingRawMessage (replace with actual logic later)
impl Handler<IncomingRawMessage> for EventActor {
    type Result = ();
    fn handle(&mut self, msg: IncomingRawMessage, _ctx: &mut Context<Self>) {
         log::debug!("Placeholder Event Actor received raw message: {}...", msg.0.chars().take(100).collect::<String>());
        // In reality, this actor would parse the message, check if it's an event,
        // determine the event type, and forward it to subscribers.
    }
}
// --- End Placeholder Actors ---


// --- Supervisor Actor ---

/// Unique ID for connections managed by the supervisor.
pub type ConnectionId = u64; // Or String, UUID, etc.

/// The top-level supervisor actor.
#[derive(Debug)]
pub struct SupervisorActor {
    config: Option<config::Config>, // Use qualified path
    next_connection_id: ConnectionId,
    // Store recipients for status updates, mapping ID to Recipient
    // Storing Addr<ConnectionActor<T>> directly is hard due to the generic T.
    // Store the recipient which doesn't have the generic type parameter problem.
    connections: HashMap<ConnectionId, Recipient<ConnectionStatusUpdate>>,
    // Store addresses of core actors needed by connections
    command_actor_addr: Option<Addr<CommandActor>>,
    event_actor_addr: Option<Addr<EventActor>>,
    // TODO: Store BrowserActor addresses, plugin manager actor etc.
}

impl SupervisorActor {
    pub fn new(config: Option<config::Config>) -> Self {
        Self {
            config,
            next_connection_id: 0,
            connections: HashMap::new(),
            command_actor_addr: None,
            event_actor_addr: None,
        }
    }

    // Helper to determine where incoming messages from a ConnectionActor should be routed.
    // This is a simplification; a real implementation might involve a dedicated RouterActor
    // or more sophisticated logic within CommandActor/EventActor to handle mixed streams.
    fn get_message_handler_recipient(&self) -> Recipient<IncomingRawMessage> {
        // TODO: Implement proper routing logic. For now, send everything to CommandActor.
        // This assumes CommandActor is responsible for differentiating responses vs events initially.
        self.command_actor_addr.as_ref()
            .expect("CommandActor address not available in supervisor")
            .clone()
            .recipient()
    }
}

impl Actor for SupervisorActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        log::info!("SupervisorActor started.");
        // Start core actors needed immediately
        log::info!("Starting core actors...");
        self.command_actor_addr = Some(CommandActor{}.start());
        self.event_actor_addr = Some(EventActor{}.start());
        log::info!("CommandActor and EventActor started.");

        // TODO: Load plugins, initialize monitoring, etc. based on config

        // TODO: Implement supervision strategies for core actors if needed.
        // By default, actix restarts actors on panic unless configured otherwise.

        // Register SupervisorActor to handle system signals like SIGINT/SIGTERM for graceful shutdown
        // Example: ctx.signals().registry().register(Signal::Interrupt, ctx.address().recipient());
        // Requires actix `signal` feature
    }

    fn stopping(&mut self, _ctx: &mut Self::Context) -> Running {
        log::info!("SupervisorActor stopping...");
        // TODO: Initiate graceful shutdown of child actors (Connections, BrowserActors etc.)
        // Send stop messages to CommandActor, EventActor etc.
        Running::Stop
    }
}

// --- Supervisor Messages ---

/// Message to request launching and managing a new transport connection.
#[derive(Message)]
#[rtype(result = "Result<ConnectionId, CoreError>")] // Use CoreError
pub struct LaunchConnection {
    pub params: ConnectParams,
    // pub owner_id: String, // Optional: Identifier for what owns this connection (e.g., browser instance ID)
}

// --- Supervisor Handlers ---

impl Handler<LaunchConnection> for SupervisorActor {
    type Result = Result<ConnectionId, CoreError>; // Use CoreError

    fn handle(&mut self, msg: LaunchConnection, ctx: &mut Context<Self>) -> Self::Result {
        let params = msg.params;
        log::info!("Supervisor handling LaunchConnection request for URL: {}", params.url);

        // 1. Validate URL (parsing happens in create_transport_actor now)
        // Optional: Pre-validate scheme here if desired before calling factory

        let connection_id = self.next_connection_id;
        self.next_connection_id += 1;

        // 2. Get Handler Recipient for Incoming Messages
        let message_handler_recipient = self.get_message_handler_recipient();

        // 3. Get Supervisor Recipient (for status updates back to self)
        let supervisor_recipient = ctx.address().recipient::<ConnectionStatusUpdate>();

        // 4. Call the factory function from janus-transport to create and start the actor
        log::info!("Requesting transport actor creation for ID: {}", connection_id);

        // Use the factory function, passing the ID
        // Note: create_transport_actor now returns Result<Addr<ConnectionActor<WebSocketTransport>>, TransportError>
        // We need to map TransportError to CoreError.
        let connection_addr = create_transport_actor(
            connection_id,
            params.clone(), // Clone ConnectParams for the factory
            message_handler_recipient,
            Some(supervisor_recipient), // Pass supervisor recipient for status updates
        ).map_err(CoreError::Transport)?; // Map TransportError -> CoreError::Transport

        log::info!("Transport actor (ID: {}) successfully started. Addr: {:?}", connection_id, connection_addr);

        // Store the recipient for status updates, associated with the ID
        self.connections.insert(connection_id, connection_addr.recipient::<ConnectionStatusUpdate>());

        Ok(connection_id) // Return the ID on success
    }
}

// Handler for status updates coming FROM ConnectionActors managed by this supervisor
impl Handler<ConnectionStatusUpdate> for SupervisorActor {
    type Result = ();

    fn handle(&mut self, msg: ConnectionStatusUpdate, _ctx: &mut Context<Self>) {
        // Message now contains the ID: msg = ConnectionStatusUpdate { id: ConnectionId, state: ConnectionState }
        let connection_id = msg.id;
        let new_state = msg.state;

        log::info!("Supervisor received status update for Connection ID {}: {:?}", connection_id, new_state);

        // Check if we are actually tracking this connection ID
        if !self.connections.contains_key(&connection_id) {
            log::warn!("Received status update for unknown or already removed Connection ID: {}", connection_id);
            return;
        }

        match new_state {
            ConnectionState::Disconnected(ref maybe_error) => {
                log::warn!("Connection ID {} has disconnected.", connection_id);
                if let Some(error) = maybe_error {
                    log::error!("Disconnection reason for ID {}: {}", connection_id, error);
                }
                // Remove the connection recipient from the map
                if self.connections.remove(&connection_id).is_some() {
                    log::info!("Removed connection ID {} from supervisor map.", connection_id);
                } else {
                    // Should not happen due to the contains_key check, but good to log
                    log::warn!("Attempted to remove connection ID {} but it was not found (race condition?).", connection_id);
                }
                // TODO: Notify the original owner/requester of this connection ID if applicable.
            }
            ConnectionState::Connected => {
                 log::info!("Connection ID {} is now connected.", connection_id);
                 // Potentially notify owner.
            }
            _ => { /* Connecting, Disconnecting - informational logging handled by the ConnectionActor */ }
        }
    }
}


// --- Old LaunchBrowser message and placeholders removed ---
// If BrowserActor logic needs to be initiated by the supervisor,
// a new message like `LaunchBrowserInstance` would be added, handled here,
// which might internally call `LaunchConnection`.

/// Message sent to actors to control their lifecycle
#[derive(Message, Debug)]
#[rtype(result = "()")]
pub enum LifecycleMessage {
    Start,
    Stop,
    Restart,
}

/// Trait for actor state representation
pub trait ActorState: std::fmt::Debug + Clone + PartialEq {
    fn to_string(&self) -> String;
}

/// Message sent to supervisor when an actor's state changes
#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct ActorStateUpdate<S: ActorState> {
    pub actor_id: String,
    pub state: S,
}

/// Trait for actor metrics
pub trait ActorMetrics {
    fn message_count(&self) -> u64;
    fn error_count(&self) -> u64;
    fn last_message_time(&self) -> Option<std::time::SystemTime>;
    fn last_error_time(&self) -> Option<std::time::SystemTime>;
}

pub struct BrowserActor {
    transport: Option<Addr<ConnectionActor<WebSocketTransport>>>,
}

impl BrowserActor {
    pub fn new() -> Self {
        Self {
            transport: None,
        }
    }
}

impl Actor for BrowserActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        log::info!("Browser actor started");
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        log::info!("Browser actor stopped");
    }
}

#[derive(Message)]
#[rtype(result = "Result<(), CoreError>")]
pub struct StartBrowser {
    pub url: String,
}

impl Handler<StartBrowser> for BrowserActor {
    type Result = Result<(), CoreError>;

    fn handle(&mut self, msg: StartBrowser, _ctx: &mut Context<Self>) -> Self::Result {
        log::info!("Starting browser with URL: {}", msg.url);
        Ok(())
    }
}

#[derive(Message)]
#[rtype(result = "Result<(), CoreError>")]
pub struct StopBrowser;

impl Handler<StopBrowser> for BrowserActor {
    type Result = Result<(), CoreError>;

    fn handle(&mut self, _msg: StopBrowser, _ctx: &mut Context<Self>) -> Self::Result {
        log::info!("Stopping browser");
        Ok(())
    }
}
