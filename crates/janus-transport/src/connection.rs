use crate::error::TransportError;
use crate::traits::Transport;
use crate::types::ConnectParams;
use crate::factory::create_transport;
use actix::prelude::*;
use log::{debug, error, info, warn};
use std::time::Duration;
use tokio::sync::mpsc;


/// Actor responsible for managing a single underlying transport connection.
///
/// It handles the connection lifecycle (connecting, disconnecting), manages
/// the read/write tasks for the transport, forwards incoming messages,
/// accepts outgoing messages, and reports status changes to its supervisor.
pub struct ConnectionActor {
    params: ConnectParams,
    state: ConnectionState,
    // Recipient for successfully received messages (e.g., CommandActor/EventActor dispatcher)
    message_handler: Recipient<IncomingMessage>,
    // Channel for sending outgoing messages to the write task
    outgoing_tx: Option<mpsc::Sender<String>>,
    // Supervisor or parent actor for reporting critical errors/state changes
    supervisor: Recipient<ConnectionStatusUpdate>,
    // Handle to the connection task, allowing it to be aborted
    connection_task: Option<SpawnHandle>,
}

impl ConnectionActor {
    pub fn new(
        params: ConnectParams,
        message_handler: Recipient<IncomingMessage>,
        supervisor: Recipient<ConnectionStatusUpdate>,
    ) -> Self {
        ConnectionActor {
            params,
            state: ConnectionState::Idle,
            message_handler,
            supervisor,
            outgoing_tx: None,
            connection_task: None,
        }
    }

    /// Helper to initiate the connection process.
    fn start_connection_task(&mut self, ctx: &mut Context<Self>) {
        if self.connection_task.is_some() || self.state == ConnectionState::Connecting || self.state == ConnectionState::Connected {
            warn!("Connection task already running or actor in active state ({:?}). Ignoring start request.", self.state);
            return;
        }

        self.state = ConnectionState::Connecting;
        info!("ConnectionActor state -> Connecting ({})", self.params.url);
        self.notify_supervisor(self.state.clone());

        // Use factory function from transport crate
        let transport_builder_result = create_transport(&self.params);

        let addr = ctx.address();
        let message_handler = self.message_handler.clone();
        let supervisor_recipient = self.supervisor.clone();
        let connect_timeout = self.params.connection_timeout;

        // Channel for sending messages to the transport write task
        let (outgoing_tx, mut outgoing_rx) = mpsc::channel::<String>(100); // Configurable buffer size?
        self.outgoing_tx = Some(outgoing_tx);


        let connection_fut = async move {
             let transport_builder = match transport_builder_result {
                Ok(builder) => builder,
                Err(e) => {
                    error!("Failed to create transport for {}: {}", addr.connected(), e);
                    // Use do_send to update state asynchronously from within the future
                    addr.do_send(TransportEvent::FailedToStart(e));
                    return;
                }
            };

            info!("Attempting to connect transport...");
            // Wrap connect attempt in a timeout
            match tokio::time::timeout(connect_timeout, connect_internal(transport_builder)).await {
                Ok(Ok(mut transport)) => {
                    info!("Transport connected successfully.");
                    addr.do_send(TransportEvent::Connected);

                    // Read loop
                    let read_task = async {
                        loop {
                            tokio::select! {
                                biased; // Prioritize checking channel closure
                                _ = addr.closed() => {
                                    info!("ConnectionActor address closed, shutting down read task.");
                                    break;
                                }
                                msg_result = transport.receive() => {
                                     match msg_result {
                                        Some(Ok(msg)) => {
                                            debug!("Received message: {}", msg); // Reduce log level
                                            if message_handler.do_send(IncomingMessage(msg)).is_err() {
                                                error!("Message handler recipient disconnected. Stopping read task.");
                                                addr.do_send(TransportEvent::Disconnected(Some(TransportError::Other("Message handler disconnected".into()))));
                                                break;
                                            }
                                        }
                                        Some(Err(e)) => {
                                            error!("Transport receive error: {}", e);
                                            addr.do_send(TransportEvent::Disconnected(Some(e)));
                                            break; // Exit read loop on error
                                        }
                                        None => {
                                            info!("Transport connection closed gracefully by remote.");
                                            addr.do_send(TransportEvent::Disconnected(None)); // None indicates graceful close
                                            break; // Exit loop on graceful close
                                        }
                                    }
                                }
                            }
                        }
                         info!("Transport read loop finished.");
                    };

                    // Write loop
                    let write_task = async {
                        loop {
                             tokio::select! {
                                biased;
                                _ = addr.closed() => {
                                    info!("ConnectionActor address closed, shutting down write task.");
                                    break;
                                }
                                maybe_msg_to_send = outgoing_rx.recv() => {
                                    if let Some(msg_to_send) = maybe_msg_to_send {
                                        debug!("Sending message: {}", msg_to_send); // Reduce log level
                                         if let Err(e) = transport.send(&msg_to_send).await {
                                            error!("Transport send error: {}", e);
                                            // Don't necessarily break the write loop, but report error maybe?
                                            // If send fails critically, the read side will likely detect disconnection.
                                            // Or signal disconnection directly:
                                            addr.do_send(TransportEvent::Disconnected(Some(e)));
                                            break; // Exit write loop on critical send error
                                         }
                                    } else {
                                        info!("Outgoing message channel closed, ending write task.");
                                        break; // Channel closed, exit loop
                                    }
                                }
                            }
                        }
                        info!("Transport write loop finished.");
                        // Ensure transport disconnect is called when write loop ends (gracefully or on error)
                        if let Err(e) = transport.disconnect().await {
                            warn!("Error during transport disconnect: {}", e);
                        }
                    };

                    // Run both tasks concurrently until one finishes (which should signal disconnection)
                    tokio::select! {
                        _ = read_task => { info!("Read task completed."); },
                        _ = write_task => { info!("Write task completed."); },
                         _ = addr.closed() => {
                             info!("ConnectionActor address closed, terminating connection task.");
                         }
                    }

                },
                Ok(Err(e)) => { // connect_internal returned an error
                    error!("Transport connect internal error: {}", e);
                    addr.do_send(TransportEvent::Disconnected(Some(e)));
                }
                Err(_) => { // Timeout occurred
                     error!("Transport connection timed out after {:?}", connect_timeout);
                     addr.do_send(TransportEvent::Disconnected(Some(TransportError::Timeout)));
                 }
            }
            info!("Connection task finished.");
        };

        // Spawn the connection logic
        self.connection_task = Some(ctx.spawn(connection_fut));
    }


    fn notify_supervisor(&self, state: ConnectionState) {
        if self.supervisor.do_send(ConnectionStatusUpdate(state)).is_err() {
            warn!("Failed to send status update to supervisor. It might have stopped.");
        }
    }

    fn stop_connection_task(&mut self) {
        if let Some(handle) = self.connection_task.take() {
            info!("Aborting connection task.");
            // Abort the task handle
            // Note: Requires specific actor framework support or manual tracking.
            // In actix, ctx.cancel_future(handle) is used.
            // For raw tokio tasks spawned in ctx.spawn, there isn't direct cancellation.
            // Rely on checking addr.closed() within the loop and closing the outgoing_tx.
            // Or store the JoinHandle if using tokio::spawn directly and call abort().
             warn!("Explicit task cancellation not directly implemented here, relying on actor stopping / channel closing.");
        }
         // Close the outgoing channel to signal the write task to stop gracefully
        if let Some(tx) = self.outgoing_tx.take() {
             drop(tx); // Dropping sender closes the channel
             info!("Outgoing message channel closed.");
        }
    }
}

// Internal helper to isolate the transport.connect() call
async fn connect_internal(mut transport: Box<dyn Transport>) -> Result<Box<dyn Transport>, TransportError> {
    transport.connect().await?;
    Ok(transport)
}


/// Represents the lifecycle state of the connection managed by `ConnectionActor`.
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Idle,
    Connecting,
    Connected,
    Disconnecting,
    Disconnected(Option<TransportError>), // Some(err) for error, None for graceful close
    FailedToStart(TransportError), // Initial creation/startup failure
}

// --- Actor Messages ---

/// Message to send a string out via the connection.
#[derive(Message, Clone)]
#[rtype(result = "Result<(), TransportError>")]
pub struct SendMessage(pub String);

/// Message received from the transport, to be forwarded to the designated handler.
#[derive(Message)]
#[rtype(result = "()")]
pub struct IncomingMessage(pub String);

/// Internal message used by the connection task to update the actor's state.
#[derive(Message)]
#[rtype(result = "()")]
enum TransportEvent {
    Connected,
    Disconnected(Option<TransportError>),
    FailedToStart(TransportError),
}

/// Message sent *to* the supervisor/parent actor to report status changes.
#[derive(Message, Debug, Clone)]
#[rtype(result = "()")]
pub struct ConnectionStatusUpdate(pub ConnectionState);


// --- Actor Implementation ---

impl Actor for ConnectionActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("ConnectionActor starting for {}", self.params.url);
        // Automatically attempt connection on start
        self.start_connection_task(ctx);
    }

    fn stopping(&mut self, _ctx: &mut Self::Context) -> Running {
        info!("ConnectionActor stopping.");
        self.stop_connection_task(); // Ensure task and channel are cleaned up

        // Update state if not already disconnected/failed
        if !matches!(self.state, ConnectionState::Disconnected(_) | ConnectionState::FailedToStart(_)) {
             self.state = ConnectionState::Disconnecting;
             info!("ConnectionActor state -> Disconnecting");
             self.notify_supervisor(self.state.clone());
        }
        Running::Stop
    }
}


// --- Message Handlers ---

// Handler for internal state updates from the connection task
impl Handler<TransportEvent> for ConnectionActor {
    type Result = ();

    fn handle(&mut self, msg: TransportEvent, ctx: &mut Context<Self>) {
        let new_state = match msg {
            TransportEvent::Connected => ConnectionState::Connected,
            TransportEvent::Disconnected(err_opt) => ConnectionState::Disconnected(err_opt),
            TransportEvent::FailedToStart(err) => ConnectionState::FailedToStart(err),
        };

        if self.state == new_state {
            debug!("Ignoring redundant state update: {:?}", new_state);
            return; // Avoid redundant updates
        }

        info!("Connection state changing from {:?} -> {:?}", self.state, new_state);
        self.state = new_state.clone();

        // Notify supervisor about the state change
        self.notify_supervisor(new_state.clone());

        // If disconnected or failed, stop the actor
        match self.state {
            ConnectionState::Disconnected(_) | ConnectionState::FailedToStart(_) => {
                warn!("ConnectionActor stopping due to state: {:?}", self.state);
                self.stop_connection_task(); // Ensure cleanup again
                ctx.stop();
            }
            ConnectionState::Connected => {
                 info!("ConnectionActor reached Connected state.");
                 // Connection is up, ready for messages.
             }
             _ => {} // Connecting, Disconnecting, Idle handled elsewhere
        }
    }
}


// Handler for sending messages *out* through the connection
impl Handler<SendMessage> for ConnectionActor {
    // Use ResponseFuture for async handling within handler
    type Result = ResponseFuture<Result<(), TransportError>>;

    fn handle(&mut self, msg: SendMessage, _ctx: &mut Context<Self>) -> Self::Result {
         let current_state = self.state.clone(); // Clone state for async block
         let maybe_tx = self.outgoing_tx.clone(); // Clone sender handle

        Box::pin(async move {
            match current_state {
                ConnectionState::Connected => {
                    if let Some(tx) = maybe_tx {
                        // Send to the mpsc channel consumed by the write task
                        if tx.send(msg.0).await.is_ok() {
                            Ok(())
                        } else {
                            error!("Outgoing message channel closed unexpectedly while connected.");
                            Err(TransportError::NotConnected("Message channel closed".into()))
                        }
                    } else {
                         error!("Attempted to send message but outgoing channel is missing (state: Connected).");
                         Err(TransportError::NotConnected("Internal channel missing".into()))
                    }
                },
                _ => {
                    warn!("Attempted to send message while not connected (State: {:?})", current_state);
                    Err(TransportError::NotConnected(format!("Current state: {:?}", current_state)))
                }
            }
        })
    }
}
