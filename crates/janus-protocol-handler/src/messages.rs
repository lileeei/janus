//! Messages used for communication between L2 actors and core protocol actors.

use actix::prelude::*;
use futures_channel::oneshot;
use janus_core::error::InternalError;
use serde_json::Value;

/// Request sent from L2 actors (Browser/Page) to CommandActor to execute a protocol command.
#[derive(Debug)] // No Clone needed as sender passes ownership
pub struct SendCommand {
    /// Optional CDP session ID (for commands targeting specific pages/targets).
    /// None for browser-level commands.
    pub session_id: Option<String>,
    /// The protocol method name (e.g., "Page.navigate").
    pub method: String,
    /// The parameters for the method.
    pub params: Value,
    /// A one-shot channel sender to send the result back to the requester.
    pub result_tx: oneshot::Sender<CommandResult>,
}

impl Message for SendCommand {
    // The result type isn't directly used here as we use the oneshot channel.
    // We return a Result<(), InternalError> to indicate if the command could be *accepted*
    // by the CommandActor (e.g., if connected). The actual command result comes via the channel.
    type Result = Result<(), InternalError>;
}

/// Represents the result of a protocol command execution.
pub type CommandResult = Result<Value, InternalError>;

/// Represents a parsed protocol event received from the browser.
#[derive(Debug, Clone, Message)]
#[rtype(result = "()")]
pub struct ProtocolEvent {
    /// Optional CDP session ID indicating the target this event belongs to.
    pub session_id: Option<String>,
    /// The event method name (e.g., "Target.targetCreated").
    pub method: String,
    /// The event parameters.
    pub params: Value,
}

/// Message to subscribe an actor to specific protocol events.
#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct Subscribe {
    /// The event method name to subscribe to (e.g., "Target.targetCreated").
    pub event_name: String,
    /// Optional session ID to only receive events for a specific target.
    /// None subscribes to browser-level events matching the name.
    pub session_id: Option<String>,
    /// The recipient actor that will receive matching `ProtocolEvent` messages.
    pub subscriber: Recipient<ProtocolEvent>,
}

/// Message to unsubscribe an actor from protocol events.
#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct Unsubscribe {
    /// The event method name to unsubscribe from.
    pub event_name: String,
    /// Optional session ID used during subscription.
    pub session_id: Option<String>,
    /// The recipient actor to remove.
    pub subscriber: Recipient<ProtocolEvent>,
}

// Internal message for CommandActor to handle timeouts
#[derive(Debug, Message)]
#[rtype(result = "()")]
pub(crate) struct CommandTimeout(pub i64);

// Helper struct for CommandActor state
#[derive(Debug)]
pub(crate) struct PendingRequestInfo {
    pub method: String,
    pub result_tx: oneshot::Sender<CommandResult>,
    pub timeout_handle: SpawnHandle,
}

/// Structure for the JSON-RPC request object sent over the wire.
#[derive(serde::Serialize, Debug)]
struct JsonRpcRequest<'a> {
    id: i64,
    method: &'a str,
    params: &'a Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    sessionId: Option<&'a str>,
}

/// Structure for the JSON-RPC response object received over the wire.
#[derive(serde::Deserialize, Debug)]
pub struct JsonRpcResponse {
    pub id: i64,
    #[serde(default)] // Use default Option if missing
    pub result: Option<Value>,
    #[serde(default)] // Use default Option if missing
    pub error: Option<JsonRpcError>,
}

/// Structure for the JSON-RPC error object within a response.
#[derive(serde::Deserialize, Debug)]
#[allow(dead_code)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(default)] // Use default Option if missing
    pub data: Option<Value>,
}

/// Structure for incoming JSON messages (can be response or event).
#[derive(serde::Deserialize, Debug)]
pub struct IncomingJson {
    // If 'id' is present, it's a response.
    pub id: Option<i64>,
    // If 'method' is present, it's likely an event (or maybe an error response without id?).
    pub method: Option<String>,
    // Event parameters.
    #[serde(default)]
    pub params: Option<Value>,
    // Response result.
    #[serde(default)]
    pub result: Option<Value>,
    // Response or protocol error.
    #[serde(default)]
    pub error: Option<JsonRpcError>,
    // Session ID for events.
    #[serde(rename = "sessionId", default)]
    pub session_id: Option<String>,
}
