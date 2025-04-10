//! The EventActor manages event subscriptions and dispatches incoming events.

use crate::messages::{ProtocolEvent, Subscribe, Unsubscribe};
use actix::prelude::*;
use log::{debug, error, info, trace, warn};
use std::collections::{HashMap, HashSet};

// Key: (Event Name, Optional Session ID). Value: Set of subscribers.
type SubscriptionMap = HashMap<(String, Option<String>), HashSet<Recipient<ProtocolEvent>>>;

pub struct EventActor {
    subscriptions: SubscriptionMap,
}

impl Default for EventActor {
    fn default() -> Self {
        Self {
            subscriptions: HashMap::new(),
        }
    }
}

impl Actor for EventActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        info!("EventActor started.");
    }

    fn stopping(&mut self, _ctx: &mut Context<Self>) -> Running {
        info!("EventActor stopping.");
        self.subscriptions.clear(); // Clear subscriptions on stop
        Running::Stop
    }
}

// Handler for Subscribe messages
impl Handler<Subscribe> for EventActor {
    type Result = ();

    fn handle(&mut self, msg: Subscribe, _ctx: &mut Context<Self>) {
        let key = (msg.event_name.clone(), msg.session_id.clone());
        debug!(
            "Adding subscription for {:?} from {:?}",
            key, msg.subscriber
        );
        self.subscriptions
            .entry(key)
            .or_default()
            .insert(msg.subscriber);
    }
}

// Handler for Unsubscribe messages
impl Handler<Unsubscribe> for EventActor {
    type Result = ();

    fn handle(&mut self, msg: Unsubscribe, _ctx: &mut Context<Self>) {
        let key = (msg.event_name.clone(), msg.session_id.clone());
        debug!(
            "Removing subscription for {:?} from {:?}",
            key, msg.subscriber
        );
        if let Some(subscribers) = self.subscriptions.get_mut(&key) {
            subscribers.remove(&msg.subscriber);
            // Remove the key entirely if no subscribers are left
            if subscribers.is_empty() {
                self.subscriptions.remove(&key);
            }
        }
    }
}

// Handler for ProtocolEvent messages (received from CommandActor)
impl Handler<ProtocolEvent> for EventActor {
    type Result = ();

    fn handle(&mut self, event: ProtocolEvent, _ctx: &mut Context<Self>) {
        trace!("EventActor received event: {:?}", event);

        // Find subscribers matching the specific event name and session ID
        let specific_key = (event.method.clone(), event.session_id.clone());
        // Find subscribers matching the event name but for *any* session ID (wildcard)
        let wildcard_key = (event.method.clone(), None);

        let mut recipients_to_notify = HashSet::new();

        if let Some(recipients) = self.subscriptions.get(&specific_key) {
            recipients_to_notify.extend(recipients.iter().cloned());
        }
        // Only add wildcard recipients if the subscription key is different from the specific one
        if specific_key != wildcard_key {
            if let Some(recipients) = self.subscriptions.get(&wildcard_key) {
                recipients_to_notify.extend(recipients.iter().cloned());
            }
        }

        if recipients_to_notify.is_empty() {
            trace!("No subscribers found for event: {:?}", event.method);
            return;
        }

        debug!(
            "Dispatching event '{}' (session: {:?}) to {} subscribers.",
            event.method,
            event.session_id,
            recipients_to_notify.len()
        );

        // Send the event to all matched subscribers
        for recipient in recipients_to_notify {
            // Use do_send for fire-and-forget. If a recipient is dead, log error.
            if recipient.do_send(event.clone()).is_err() {
                warn!(
                    "Failed to send event {:?} to subscriber {:?}. It might have stopped. Consider unsubscribing.",
                    event.method, recipient
                );
                // TODO: Add mechanism to automatically unsubscribe dead actors? Complex.
            }
        }
    }
}
