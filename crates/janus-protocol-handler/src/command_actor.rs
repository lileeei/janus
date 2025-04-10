//! The CommandActor handles sending commands, tracking responses, and managing timeouts.

use crate::messages::{
    CommandResult, CommandTimeout, IncomingJson, JsonRpcError, JsonRpcRequest, JsonRpcResponse,
    PendingRequestInfo, ProtocolEvent, SendCommand,
};
use actix::prelude::*;
use janus_core::{error::InternalError, Config};
use janus_transport::{ConnectionActor, IncomingMessage, SendMessage};
use log::{debug, error, info, trace, warn};
use std::{collections::HashMap, time::Duration};

pub struct CommandActor {
    config: Config,
    connection_actor: Addr<ConnectionActor>,
    event_actor: Recipient<ProtocolEvent>, // Where to forward events
    next_id: i64,
    pending_requests: HashMap<i64, PendingRequestInfo>,
}

impl CommandActor {
    pub fn new(
        config: Config,
        connection_actor: Addr<ConnectionActor>,
        event_actor: Recipient<ProtocolEvent>,
    ) -> Self {
        Self {
            config,
            connection_actor,
            event_actor,
            next_id: 1,
            pending_requests: HashMap::new(),
        }
    }

    fn handle_response(&mut self, response: JsonRpcResponse, ctx: &mut Context<Self>) {
        if let Some(pending) = self.pending_requests.remove(&response.id) {
            // Cancel the timeout future
            ctx.cancel_future(pending.timeout_handle);

            let result: CommandResult = if let Some(error) = response.error {
                Err(InternalError::Protocol {
                    code: Some(error.code),
                    message: error.message,
                    data: error.data.map(|v| v.to_string()),
                })
            } else {
                Ok(response.result.unwrap_or(Value::Null)) // Return Null if result is omitted
            };

            // Send the result back to the original requester
            if pending.result_tx.send(result).is_err() {
                // This is expected if the requester dropped the future (e.g., timed out itself)
                debug!(
                    "Requester for command id {} (method: {}) dropped the result channel.",
                    response.id, pending.method
                );
            }
        } else {
            warn!(
                "Received response for unknown or already handled command id: {}",
                response.id
            );
        }
    }

    fn handle_event(
        &self,
        session_id: Option<String>,
        method: String,
        params: Option<Value>,
    ) {
        let event = ProtocolEvent {
            session_id,
            method,
            params: params.unwrap_or(Value::Null),
        };
        if self.event_actor.do_send(event).is_err() {
            error!("Failed to forward event to EventActor (it might have stopped).");
        }
    }
}

impl Actor for CommandActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        info!("CommandActor started.");
        // Potentially subscribe self to IncomingMessage from ConnectionActor?
        // This assumes ConnectionActor is configured to send IncomingMessage to CommandActor.
    }

    fn stopping(&mut self, ctx: &mut Context<Self>) -> Running {
        info!("CommandActor stopping.");
        // Cancel all pending requests and notify requesters with an error
        for (id, pending) in self.pending_requests.drain() {
            ctx.cancel_future(pending.timeout_handle);
            let _ = pending.result_tx.send(Err(InternalError::Actor(
                "CommandActor shut down".to_string(),
            )));
            debug!("Cancelled pending command id {} due to CommandActor stopping.", id);
        }
        Running::Stop
    }
}

// Handler for SendCommand requests from L2 actors
impl Handler<SendCommand> for CommandActor {
    type Result = Result<(), InternalError>; // Immediate result: command accepted or rejected

    fn handle(&mut self, msg: SendCommand, ctx: &mut Context<Self>) -> Self::Result {
        let command_id = self.next_id;
        self.next_id += 1;

        let command_method = msg.method.clone(); // Clone for logging/storage

        // Serialize the JSON-RPC request
        let request = JsonRpcRequest {
            id: command_id,
            method: &msg.method,
            params: &msg.params,
            sessionId: msg.session_id.as_deref(),
        };

        let json_request = match serde_json::to_string(&request) {
            Ok(json) => json,
            Err(e) => {
                error!("Failed to serialize command {}: {}", command_id, e);
                // Send error immediately via oneshot channel, no need to store pending request
                let _ = msg.result_tx.send(Err(InternalError::Serialization(e.to_string())));
                // Return Ok() because the SendCommand message *was* handled, even if it failed internally.
                // Alternatively, return Err() here to signal acceptance failure. Let's return Err.
                return Err(InternalError::Serialization(e.to_string()));
            }
        };

        trace!("Sending command ({}): {}", command_id, json_request);

        // Store pending request info before sending
        let timeout_duration = self.config.global.default_command_timeout; // Use configured timeout
        let timeout_handle =
            ctx.notify_later(CommandTimeout(command_id), timeout_duration);

        let pending_info = PendingRequestInfo {
            method: command_method,
            result_tx: msg.result_tx,
            timeout_handle,
        };
        self.pending_requests.insert(command_id, pending_info);


        // Send the message via ConnectionActor
        // Use `do_send` for fire-and-forget, or `send` if we need to handle transport errors immediately.
        // If `send` fails, we need to clean up the pending request.
        let send_future = self.connection_actor.send(SendMessage(json_request));

        // Handle the result of sending asynchronously
        let future = async move {
            match send_future.await {
                Ok(Ok(())) => {
                    // Send successful
                    trace!("Command {} sent successfully to transport.", command_id);
                }
                Ok(Err(transport_err)) => {
                    // Transport layer rejected the send
                    error!(
                        "Transport error sending command {}: {}",
                        command_id, transport_err
                    );
                    // Need to inform the original requester and clean up
                    return Some(Err(InternalError::Transport(transport_err))); // Signal cleanup needed
                }
                Err(mailbox_err) => {
                    // Failed to send message to ConnectionActor
                    error!(
                        "Mailbox error sending command {} to ConnectionActor: {}",
                        command_id, mailbox_err
                    );
                     return Some(Err(InternalError::Actor(format!(
                         "ConnectionActor mailbox error: {}",
                         mailbox_err
                     )))); // Signal cleanup needed
                }
            }
             None // No cleanup needed
        }.into_actor(self)
         .map(move |error_result, actor, ctx| {
              if let Some(Err(err)) = error_result {
                 // If sending failed, remove the pending request and notify the requester
                 if let Some(pending) = actor.pending_requests.remove(&command_id) {
                     ctx.cancel_future(pending.timeout_handle);
                     let _ = pending.result_tx.send(Err(err)); // Forward the error
                 }
             }
         });

        ctx.spawn(future);

        Ok(()) // Command accepted for processing
    }
}

// Handler for IncomingMessage from ConnectionActor
impl Handler<IncomingMessage> for CommandActor {
    type Result = ();

    fn handle(&mut self, msg: IncomingMessage, ctx: &mut Context<Self>) {
        trace!("CommandActor received raw message: {}", msg.0);
        match serde_json::from_str::<IncomingJson>(&msg.0) {
            Ok(parsed) => {
                if let Some(id) = parsed.id {
                    // This is a response
                    let response = JsonRpcResponse {
                        id,
                        result: parsed.result,
                        error: parsed.error,
                    };
                    self.handle_response(response, ctx);
                } else if let Some(method) = parsed.method {
                    // This is an event
                    self.handle_event(parsed.session_id, method, parsed.params);
                } else {
                    // Neither response nor event, could be a protocol error without an ID
                    // or just unexpected JSON.
                    if let Some(err_obj) = parsed.error {
                         warn!(
                             "Received JSON-RPC error without ID: Code={}, Msg='{}'",
                             err_obj.code, err_obj.message
                         );
                         // Could potentially forward this as a generic error event?
                         self.handle_event(None, "Protocol.error".to_string(), Some(serde_json::to_value(err_obj).unwrap_or_default()));
                    } else {
                         warn!("Received unexpected JSON message: {}", msg.0);
                    }
                }
            }
            Err(e) => {
                error!("Failed to deserialize incoming message: {}. Raw: {}", e, msg.0);
                // Handle deserialization error - maybe send to EventActor as a special event?
                self.handle_event(None, "Protocol.deserializeError".to_string(), Some(json!({ "error": e.to_string(), "raw": msg.0 })));
            }
        }
    }
}

// Handler for internal CommandTimeout messages
impl Handler<CommandTimeout> for CommandActor {
    type Result = ();

    fn handle(&mut self, msg: CommandTimeout, _ctx: &mut Context<Self>) {
        let command_id = msg.0;
        if let Some(pending) = self.pending_requests.remove(&command_id) {
            warn!(
                "Command id {} (method: {}) timed out.",
                command_id, pending.method
            );
            // Send timeout error back to the requester
            let _ = pending.result_tx.send(Err(InternalError::Timeout));
        }
        // No need to cancel future, it already fired.
    }
}

// Make CommandActor process IncomingMessage (needs registration or direct sending)
impl Handler<ConnectionStatusUpdate> for CommandActor {
    type Result = ();

    fn handle(&mut self, msg: ConnectionStatusUpdate, ctx: &mut Context<Self>) {
        info!("CommandActor received ConnectionStatusUpdate: {:?}", msg.0);
        // If the connection drops, we might want to fail pending commands
        if let ConnectionState::Disconnected(Some(err)) = msg.0 {
            warn!("Connection dropped! Failing all pending commands.");
            for (id, pending) in self.pending_requests.drain() {
                ctx.cancel_future(pending.timeout_handle);
                let _ = pending.result_tx.send(Err(InternalError::Transport(err.clone())));
                debug!("Cancelled pending command id {} due to connection drop.", id);
            }
        }
    }
}
