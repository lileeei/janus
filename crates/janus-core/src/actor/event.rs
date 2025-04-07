use std::collections::{HashMap, HashSet};
use actix::{Actor, Addr, Context, Handler, Recipient, Supervised};
use log::{error, info, warn};
use serde_json::Value;

use crate::error::{CoreError, ProtocolError};
use super::{
    ActorConfig, ActorError, ActorMetrics, ActorState,
    messages::{IncomingRawMessage, LifecycleMessage, ProtocolEvent, Subscribe, SubscriptionId, Unsubscribe, SupervisionMessage},
    supervisor::SupervisorActor,
};

#[derive(Debug)]
pub enum EventState {
    Ready,
    Failed(ProtocolError),
}

impl ActorState for EventState {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Ready => "Ready",
            Self::Failed(_) => "Failed",
        }
    }
}

#[derive(Debug, Clone)]
pub struct EventConfig {
    pub buffer_size: usize,
}

impl Default for EventConfig {
    fn default() -> Self {
        Self {
            buffer_size: 1000,
        }
    }
}

impl ActorConfig for EventConfig {
    fn validate(&self) -> Result<(), ActorError> {
        if self.buffer_size == 0 {
            return Err(ActorError::InitializationError(
                "Buffer size must be greater than 0".to_string(),
            ));
        }
        Ok(())
    }
}

pub struct EventActor {
    config: EventConfig,
    state: EventState,
    supervisor: Addr<SupervisorActor>,
    subscribers: HashMap<String, HashMap<SubscriptionId, Recipient<ProtocolEvent>>>,
    next_subscription_id: u64,
    metrics: EventMetrics,
}

#[derive(Debug, Default)]
struct EventMetrics {
    events_received: u64,
    events_delivered: u64,
    delivery_errors: u64,
    active_subscriptions: usize,
    last_event_at: Option<std::time::SystemTime>,
    last_error_at: Option<std::time::SystemTime>,
}

impl Actor for EventActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("EventActor started");
        
        // Register with supervisor
        self.supervisor.do_send(SupervisionMessage::RegisterChild {
            actor_type: "event",
            id: "main".to_string(),
        });
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        info!("EventActor stopped");
        self.subscribers.clear();
    }
}

impl Supervised for EventActor {
    fn restarting(&mut self, _ctx: &mut <Self as Actor>::Context) {
        warn!("EventActor is being restarted");
        self.subscribers.clear();
    }
}

impl EventActor {
    pub fn new(config: EventConfig, supervisor: Addr<SupervisorActor>) -> Result<Self, ActorError> {
        config.validate()?;
        Ok(Self {
            config,
            state: EventState::Ready,
            supervisor,
            subscribers: HashMap::new(),
            next_subscription_id: 1,
            metrics: EventMetrics::default(),
        })
    }

    fn dispatch_event(&mut self, event: ProtocolEvent) {
        if let Some(subscribers) = self.subscribers.get(&event.event_type) {
            let mut failed_subscriptions = HashSet::new();
            
            for (id, subscriber) in subscribers {
                if let Err(e) = subscriber.do_send(event.clone()) {
                    error!("Failed to deliver event to subscriber {}: {}", id.0, e);
                    failed_subscriptions.insert(*id);
                    self.metrics.delivery_errors += 1;
                    self.metrics.last_error_at = Some(std::time::SystemTime::now());
                } else {
                    self.metrics.events_delivered += 1;
                }
            }

            // Clean up failed subscriptions
            if !failed_subscriptions.is_empty() {
                if let Some(subs) = self.subscribers.get_mut(&event.event_type) {
                    for id in failed_subscriptions {
                        subs.remove(&id);
                    }
                }
                self.update_subscription_metrics();
            }
        }
    }

    fn update_subscription_metrics(&mut self) {
        self.metrics.active_subscriptions = self.subscribers.values()
            .map(|subs| subs.len())
            .sum();
    }
}

impl Handler<Subscribe> for EventActor {
    type Result = Result<SubscriptionId, CoreError>;

    fn handle(&mut self, msg: Subscribe, _ctx: &mut Context<Self>) -> Self::Result {
        let id = SubscriptionId(self.next_subscription_id);
        self.next_subscription_id += 1;

        self.subscribers
            .entry(msg.event_type)
            .or_default()
            .insert(id, msg.subscriber);

        self.update_subscription_metrics();
        
        Ok(id)
    }
}

impl Handler<Unsubscribe> for EventActor {
    type Result = Result<(), CoreError>;

    fn handle(&mut self, msg: Unsubscribe, _ctx: &mut Context<Self>) -> Self::Result {
        for subscribers in self.subscribers.values_mut() {
            subscribers.remove(&msg.subscription_id);
        }

        // Clean up empty event types
        self.subscribers.retain(|_, subs| !subs.is_empty());
        self.update_subscription_metrics();
        
        Ok(())
    }
}

impl Handler<IncomingRawMessage> for EventActor {
    type Result = ();

    fn handle(&mut self, msg: IncomingRawMessage, _ctx: &mut Context<Self>) {
        match serde_json::from_str::<Value>(&msg.payload) {
            Ok(value) => {
                // Only handle messages without an ID (events)
                if value.get("id").is_none() {
                    if let Some(method) = value.get("method").and_then(Value::as_str) {
                        let event = ProtocolEvent {
                            event_type: method.to_string(),
                            data: value.get("params").cloned().unwrap_or(Value::Null),
                        };

                        self.metrics.events_received += 1;
                        self.metrics.last_event_at = Some(std::time::SystemTime::now());
                        
                        self.dispatch_event(event);
                    }
                }
            }
            Err(e) => {
                error!("Failed to parse event message: {}", e);
                self.metrics.delivery_errors += 1;
                self.metrics.last_error_at = Some(std::time::SystemTime::now());
            }
        }
    }
}

impl Handler<LifecycleMessage> for EventActor {
    type Result = ();

    fn handle(&mut self, msg: LifecycleMessage, ctx: &mut Context<Self>) {
        match msg {
            LifecycleMessage::Initialize => {
                self.state = EventState::Ready;
            }
            LifecycleMessage::Start => {
                // Nothing special needed
            }
            LifecycleMessage::Stop => {
                self.state = EventState::Failed(ProtocolError::Internal(
                    "Actor stopping".to_string(),
                ));
                self.subscribers.clear();
                ctx.stop();
            }
            LifecycleMessage::Restart => {
                self.state = EventState::Ready;
                self.subscribers.clear();
            }
        }
    }
}

impl ActorMetrics for EventActor {
    fn get_metrics(&self) -> super::ActorMetricsData {
        super::ActorMetricsData {
            message_count: self.metrics.events_received,
            error_count: self.metrics.delivery_errors,
            last_activity: self.metrics.last_event_at
                .or(self.metrics.last_error_at)
                .unwrap_or_else(|| std::time::SystemTime::now()),
        }
    }
} 