use actix::Message;
use serde_json::Value;
use std::time::Duration;

use crate::error::{CoreError, ProtocolError};

/// Message for raw protocol communication
#[derive(Message, Debug)]
#[rtype(result = "Result<(), CoreError>")]
pub struct SendRawMessage {
    pub payload: String,
    pub timeout: Option<Duration>,
}

/// Message for incoming raw protocol data
#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct IncomingRawMessage {
    pub payload: String,
}

/// Message for executing a protocol command
#[derive(Message, Debug)]
#[rtype(result = "Result<Value, ProtocolError>")]
pub struct ExecuteCommand {
    pub method: String,
    pub params: Option<Value>,
    pub timeout: Option<Duration>,
}

/// Message for protocol events
#[derive(Message, Debug, Clone)]
#[rtype(result = "()")]
pub struct ProtocolEvent {
    pub event_type: String,
    pub data: Value,
}

/// Message for actor lifecycle management
#[derive(Message, Debug)]
#[rtype(result = "()")]
pub enum LifecycleMessage {
    Initialize,
    Start,
    Stop,
    Restart,
}

/// Message for actor supervision
#[derive(Message, Debug)]
#[rtype(result = "()")]
pub enum SupervisionMessage {
    RegisterChild {
        actor_type: &'static str,
        id: String,
    },
    ChildFailed {
        actor_type: &'static str,
        id: String,
        error: CoreError,
    },
    ChildStopped {
        actor_type: &'static str,
        id: String,
    },
}

/// Message for event subscription
#[derive(Message, Debug)]
#[rtype(result = "Result<SubscriptionId, CoreError>")]
pub struct Subscribe {
    pub event_type: String,
    pub subscriber: actix::Recipient<ProtocolEvent>,
}

/// Message for event unsubscription
#[derive(Message, Debug)]
#[rtype(result = "Result<(), CoreError>")]
pub struct Unsubscribe {
    pub subscription_id: SubscriptionId,
}

/// Unique identifier for event subscriptions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriptionId(pub u64); 