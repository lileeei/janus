//! Actors specific to the Chrome browser implementation (L2).

use actix::prelude::*;
use futures_channel::oneshot;
use janus_core::error::InternalError;
use janus_interfaces::common::*; // Re-export L1 common types if needed by messages
use janus_protocol_handler::{
    CommandActor, EventActor, ProtocolEvent, SendCommand, Subscribe, Unsubscribe,
};
use log::{debug, error, info, warn};
use serde_json::Value;
use std::collections::HashMap;

use crate::protocol::*; // Import CDP structures

// ================= Messages =================

// Messages sent TO ChromeBrowserActor
#[derive(Debug, Message)]
#[rtype(result = "Result<String, InternalError>")]
pub struct GetVersion;

#[derive(Debug, Message)]
#[rtype(result = "Result<NewPageResponse, InternalError>")]
pub struct CreatePage {
    pub url: String,
}

#[derive(Debug, Message)]
#[rtype(result = "Result<Vec<PageInfo>, InternalError>")]
pub struct GetPages;

#[derive(Debug, Message)]
#[rtype(result = "()")] // Just ack stopping process begins
pub struct ShutdownBrowser;


// Response from CreatePage
#[derive(Debug)]
pub struct NewPageResponse {
    pub page_id: String, // Usually the target ID
    pub page_actor_addr: Addr<ChromePageActor>,
}

// Info about a page for GetPages
#[derive(Debug, Clone)]
pub struct PageInfo {
    pub id: String, // Target ID
    pub title: String,
    pub url: String,
    pub actor_addr: Addr<ChromePageActor>,
}


// Messages sent TO ChromePageActor
#[derive(Debug, Message)]
#[rtype(result = "Result<(), InternalError>")]
pub struct Navigate {
    pub url: String,
}

#[derive(Debug, Message)]
#[rtype(result = "Result<Value, InternalError>")]
pub struct EvaluateScript {
    pub script: String,
}

#[derive(Debug, Message)]
#[rtype(result = "Result<(), InternalError>")]
pub struct ClosePage;


// ================= Chrome Browser Actor =================

#[derive(Debug, Default)]
enum BrowserActorState {
    #[default]
    Initializing,
    DiscoveringTargets,
    Ready,
    Closing,
    Closed,
}

pub struct ChromeBrowserActor {
    state: BrowserActorState,
    command_actor: Addr<CommandActor>,
    event_actor: Recipient<ProtocolEvent>,
    // Maps Target ID -> Page Actor Address
    page_actors: HashMap<String, Addr<ChromePageActor>>,
    // Maps Target ID -> Session ID (for sending commands)
    target_sessions: HashMap<String, String>,
    // Self address for subscriptions
    self_addr: Option<Addr<Self>>,
}

impl ChromeBrowserActor {
    pub fn new(
        command_actor: Addr<CommandActor>,
        event_actor: Recipient<ProtocolEvent>,
    ) -> Self {
        Self {
            state: BrowserActorState::Initializing,
            command_actor,
            event_actor,
            page_actors: HashMap::new(),
            target_sessions: HashMap::new(),
            self_addr: None,
        }
    }

    // Helper to send a command and await the result via oneshot channel
    async fn send_command(
        &self,
        session_id: Option<String>,
        method: String,
        params: Value,
    ) -> Result<Value, InternalError> {
        let (tx, rx) = oneshot::channel();
        let command = SendCommand {
            session_id,
            method,
            params,
            result_tx: tx,
        };

        self.command_actor
            .send(command)
            .await
            .map_err(|mb_err| InternalError::Actor(format!("CommandActor mailbox error: {}", mb_err)))? // Mailbox Error
            .map_err(|accept_err| accept_err)?; // SendCommand acceptance Error (e.g. serialization)


        // Await the result from the oneshot channel
        rx.await.map_err(|_canceled| {
            InternalError::Actor("Command result channel cancelled".to_string())
        })? // Channel Canceled/Dropped Error
    }


    // Helper to subscribe to events
    fn subscribe_to_event(&self, event_name: &str, session_id: Option<String>, addr: Recipient<ProtocolEvent>) {
         debug!("BrowserActor subscribing to {} (session: {:?})", event_name, session_id);
         if self.event_actor.do_send(Subscribe {
             event_name: event_name.to_string(),
             session_id,
             subscriber: addr,
         }).is_err() {
             error!("Failed to send Subscribe message to EventActor.");
         }
    }


    // Handles Target.* events
    fn handle_target_event(&mut self, event: ProtocolEvent, ctx: &mut Context<Self>) {
        match event.method.as_str() {
            "Target.targetCreated" => {
                match serde_json::from_value::<TargetCreatedParams>(event.params) {
                    Ok(params) => {
                        info!("New target created: {:?}", params.target_info);
                        if params.target_info.type_ == "page" && !self.page_actors.contains_key(&params.target_info.target_id) {
                             // If it's a page target we don't know about, try to attach and create an actor
                             self.attach_and_create_page_actor(params.target_info.target_id, ctx);
                         }
                    }
                    Err(e) => warn!("Failed to parse Target.targetCreated params: {}", e),
                }
            }
            "Target.targetInfoChanged" => {
                 match serde_json::from_value::<TargetInfoChangedParams>(event.params) {
                    Ok(params) => {
                         debug!("Target info changed: {:?}", params.target_info);
                         // Could update page actor state if needed (e.g., URL, title)
                    }
                     Err(e) => warn!("Failed to parse Target.targetInfoChanged params: {}", e),
                 }
             }
             "Target.attachedToTarget" => {
                 // This event provides the session ID after attaching
                 #[derive(Deserialize)]
                 #[serde(rename_all = "camelCase")]
                 struct AttachedParams { session_id: String, target_info: TargetInfo }

                 match serde_json::from_value::<AttachedParams>(event.params) {
                     Ok(params) => {
                         info!("Attached to target {}, session ID: {}", params.target_info.target_id, params.session_id);
                         if params.target_info.type_ == "page" {
                             self.target_sessions.insert(params.target_info.target_id.clone(), params.session_id.clone());
                             // If we don't have an actor yet, create one now
                             if !self.page_actors.contains_key(&params.target_info.target_id) {
                                 self.create_page_actor_internal(params.target_info.target_id, params.session_id, ctx);
                             }
                         }
                     }
                     Err(e) => warn!("Failed to parse Target.attachedToTarget params: {}", e),
                 }

             }
             "Target.detachedFromTarget" => {
                 match serde_json::from_value::<DetachedFromTargetParams>(event.params) {
                    Ok(params) => {
                        info!("Detached from target session: {}", params.session_id);
                        // Find target ID associated with session ID and remove actor
                        let target_id = self.target_sessions.iter()
                            .find_map(|(tid, sid)| if sid == &params.session_id { Some(tid.clone()) } else { None });

                        if let Some(tid) = target_id {
                             if let Some(page_actor) = self.page_actors.remove(&tid) {
                                 info!("Stopping PageActor for detached target: {}", tid);
                                 page_actor.do_send(ClosePage); // Tell actor to stop gracefully
                             }
                             self.target_sessions.remove(&tid);
                        } else {
                             warn!("Received detachedFromTarget for unknown session: {}", params.session_id);
                         }
                     }
                    Err(e) => warn!("Failed to parse Target.detachedFromTarget params: {}", e),
                 }
            }
             "Target.targetDestroyed" => {
                 match serde_json::from_value::<TargetDestroyedParams>(event.params) {
                    Ok(params) => {
                        info!("Target destroyed: {}", params.target_id);
                         if let Some(page_actor) = self.page_actors.remove(&params.target_id) {
                             info!("Stopping PageActor for destroyed target: {}", params.target_id);
                             page_actor.do_send(ClosePage); // Tell actor to stop gracefully
                         }
                         self.target_sessions.remove(&params.target_id);
                     }
                    Err(e) => warn!("Failed to parse Target.targetDestroyed params: {}", e),
                 }
             }
            _ => {} // Ignore other Target.* events for now
        }
    }

    // Spawns a task to attach to a target and create its actor
    fn attach_and_create_page_actor(&self, target_id: String, ctx: &mut Context<Self>) {
         info!("Attempting to attach to target: {}", target_id);
         let command_actor = self.command_actor.clone();
         let self_addr = ctx.address(); // Use ctx.address()

         ctx.spawn(async move {
             let params = AttachToTargetParams { target_id: target_id.clone(), flatten: Some(true) };
             let command = SendCommand {
                 session_id: None, // Browser-level command
                 method: "Target.attachToTarget".to_string(),
                 params: serde_json::to_value(params).unwrap(),
                 result_tx: {
                     let (tx, rx) = oneshot::channel();
                     // Need to handle the result of attachToTarget *outside* the SendCommand
                     // because SendCommand's result_tx expects the final command result.
                     // Let's handle the response via event "Target.attachedToTarget" instead.
                     // So, we don't actually need the result here. Send dummy channel.
                     tx
                 }
             };

             // Send the attach command, ignore immediate result (wait for event)
             if command_actor.send(command).await.is_err() {
                  error!("Failed to send AttachToTarget command for target {}", target_id);
             }
             // Result (session ID) will be handled by "Target.attachedToTarget" event handler

         }.into_actor(self)); // Associate future with the actor
    }

    fn create_page_actor_internal(&mut self, target_id: String, session_id: String, ctx: &mut Context<Self>) -> Addr<ChromePageActor> {
        info!("Creating PageActor for target {}, session {}", target_id, session_id);
        let page_actor = ChromePageActor::new(
            target_id.clone(),
            session_id,
            self.command_actor.clone(),
            self.event_actor.clone(),
        ).start();
        self.page_actors.insert(target_id, page_actor.clone());
        page_actor
    }

}


impl Actor for ChromeBrowserActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        info!("ChromeBrowserActor started. Initializing...");
        self.self_addr = Some(ctx.address()); // Store own address

        // Subscribe to relevant Target.* events using own address recipient
        let self_recipient = ctx.address().recipient();
        self.subscribe_to_event("Target.targetCreated", None, self_recipient.clone());
        self.subscribe_to_event("Target.targetInfoChanged", None, self_recipient.clone());
        self.subscribe_to_event("Target.attachedToTarget", None, self_recipient.clone()); // Handle attach results
        self.subscribe_to_event("Target.detachedFromTarget", None, self_recipient.clone());
        self.subscribe_to_event("Target.targetDestroyed", None, self_recipient);

        // Enable target discovery
        let command_actor = self.command_actor.clone();
        ctx.spawn(async move {
            info!("Enabling target discovery...");
            let params = SetDiscoverTargetsParams { discover: true };
             let (tx, rx) = oneshot::channel();
             let command = SendCommand {
                 session_id: None,
                 method: "Target.setDiscoverTargets".to_string(),
                 params: serde_json::to_value(params).unwrap(),
                 result_tx: tx,
             };
             if command_actor.send(command).await.is_err() {
                 error!("Failed to send setDiscoverTargets command");
                 // TODO: Signal failure state?
                 return;
             }
             match rx.await {
                 Ok(Ok(_)) => info!("Target discovery enabled."),
                 Ok(Err(e)) => error!("Error enabling target discovery: {}", e),
                 Err(_) => error!("setDiscoverTargets channel cancelled"),
             }
             // TODO: Fetch initial targets and set state to Ready
             // actor.state = BrowserActorState::Ready; // Need to send message back to actor
        }.into_actor(self).map(|_, actor, _ctx| {
             // TODO: Transition state properly after discovery is enabled and maybe initial targets fetched
             info!("Target discovery setup complete. Actor potentially ready.");
             actor.state = BrowserActorState::Ready; // Simplification for Phase 2
        }));
        self.state = BrowserActorState::DiscoveringTargets;

    }

    fn stopping(&mut self, _ctx: &mut Context<Self>) -> Running {
        info!("ChromeBrowserActor stopping.");
        self.state = BrowserActorState::Closing;
        // Stop all managed page actors
        for page_actor in self.page_actors.values() {
            page_actor.do_send(ClosePage); // Request graceful close
        }
        self.page_actors.clear();
        self.target_sessions.clear();
        // TODO: Unsubscribe from events? Might happen automatically if EventActor handles dead recipients.
        Running::Stop
    }
}

// --- Browser Actor Message Handlers ---

impl Handler<GetVersion> for ChromeBrowserActor {
    type Result = ResponseFuture<Result<String, InternalError>>;

    fn handle(&mut self, _msg: GetVersion, _ctx: &mut Context<Self>) -> Self::Result {
        let future = self.send_command(None, "Browser.getVersion".to_string(), json!({}));

        Box::pin(async move {
            let result_value = future.await?;
            // Expect result like: {"protocolVersion": "...", "product": "...", ...}
            Ok(result_value.to_string()) // Return JSON string for simplicity
        })
    }
}

impl Handler<CreatePage> for ChromeBrowserActor {
    type Result = ResponseFuture<Result<NewPageResponse, InternalError>>;

    fn handle(&mut self, msg: CreatePage, ctx: &mut Context<Self>) -> Self::Result {
        let command_actor = self.command_actor.clone();
        let self_addr = ctx.address(); // Get self address to interact with state later

        Box::pin(async move {
            info!("BrowserActor handling CreatePage request for URL: {}", msg.url);
            let params = CreateTargetParams { url: msg.url };
            let result_value = Self::send_command(&self_addr.clone().into(), // Kludgy way to call method on self from async block
                None,
                "Target.createTarget".to_string(),
                serde_json::to_value(params).map_err(|e| InternalError::Serialization(e.to_string()))?
            ).await?;

            let create_result: CreateTargetResult = serde_json::from_value(result_value)
                .map_err(|e| InternalError::Deserialization(format!("Failed to parse CreateTargetResult: {}", e)))?;

            let target_id = create_result.target_id;
            info!("Target.createTarget successful, target_id: {}", target_id);

            // Now we need to attach to this target to get a session ID and control it.
            // The attachment and actor creation is handled via events ("Target.attachedToTarget").
            // We need to wait until the actor is created and return its address.

            // Use a temporary oneshot channel to wait for the actor creation signal
            // This is a bit complex, maybe there's a simpler way?
            // Alternative: L2 CreatePage polls GetPages until the new page appears? Less robust.
            // Let's try polling the actor's state directly via `call`.

             let check_interval = Duration::from_millis(100);
             let timeout = Duration::from_secs(10); // Timeout for page actor appearing
             let start = tokio::time::Instant::now();

             loop {
                 if start.elapsed() > timeout {
                     return Err(InternalError::Timeout);
                 }

                 // Use `call` to interact with the actor's state safely from the async block
                 if let Ok(page_actor_addr) = self_addr.call(GetPageActorAddr(target_id.clone())).await {
                     info!("Page actor found for target {}", target_id);
                    return Ok(NewPageResponse {
                         page_id: target_id,
                         page_actor_addr: page_actor_addr,
                     });
                 }

                 // Actor not found yet, wait and retry
                 tokio::time::sleep(check_interval).await;
             }
        })
    }
}


// Internal message for CreatePage handler to query state
#[derive(Message)]
#[rtype(result = "Option<Addr<ChromePageActor>>")]
struct GetPageActorAddr(String); // target_id

impl Handler<GetPageActorAddr> for ChromeBrowserActor {
    type Result = Option<Addr<ChromePageActor>>;
    fn handle(&mut self, msg: GetPageActorAddr, _ctx: &mut Context<Self>) -> Self::Result {
        self.page_actors.get(&msg.0).cloned()
    }
}


impl Handler<GetPages> for ChromeBrowserActor {
     type Result = Result<Vec<PageInfo>, InternalError>; // Directly return result

     fn handle(&mut self, _msg: GetPages, _ctx: &mut Context<Self>) -> Self::Result {
         // This just returns the currently known page actors.
         // For a more accurate list, we might need to call Target.getTargets.
         // Phase 2: Return actors we know about.
         let pages: Vec<PageInfo> = self.page_actors
             .iter()
             // .filter_map(|(tid, addr)| { // Also need URL/Title, which actor doesn't have easily
             //      // Need to ask each PageActor for its URL/Title? Too complex for now.
             //      Some(PageInfo { id: tid.clone(), title: "Unknown".into(), url: "Unknown".into(), actor_addr: addr.clone() })
             // })
             .map(|(tid, addr)| PageInfo { id: tid.clone(), title: "Unknown".into(), url: "Unknown".into(), actor_addr: addr.clone() })
             .collect();
          Ok(pages)
     }
 }

impl Handler<ShutdownBrowser> for ChromeBrowserActor {
     type Result = ();
     fn handle(&mut self, _msg: ShutdownBrowser, ctx: &mut Context<Self>) -> Self::Result {
         info!("ShutdownBrowser message received. Stopping actor and pages.");
         // TODO: Send Browser.close command?
         ctx.stop();
     }
}


// Handler for ProtocolEvent messages (forwarded by EventActor)
impl Handler<ProtocolEvent> for ChromeBrowserActor {
    type Result = ();

    fn handle(&mut self, msg: ProtocolEvent, ctx: &mut Context<Self>) {
        trace!("BrowserActor received event: {:?}", msg);
        if msg.method.starts_with("Target.") {
            self.handle_target_event(msg, ctx);
        } else {
            // Ignore other events at browser level for now
        }
    }
}


// ================= Chrome Page Actor =================

#[derive(Debug, Default)]
enum PageActorState {
    #[default]
    Initializing, // Attached, but maybe not fully loaded/ready
    Navigating,
    Idle,
    Evaluating,
    Closing,
    Closed,
}

pub struct ChromePageActor {
    target_id: String,
    session_id: String,
    state: PageActorState,
    command_actor: Addr<CommandActor>,
    event_actor: Recipient<ProtocolEvent>,
}

impl ChromePageActor {
    pub fn new(
        target_id: String,
        session_id: String,
        command_actor: Addr<CommandActor>,
        event_actor: Recipient<ProtocolEvent>,
    ) -> Self {
        Self {
            target_id,
            session_id,
            state: PageActorState::Initializing,
            command_actor,
            event_actor,
        }
    }

    // Helper to send a command *for this page's session*
    async fn send_page_command(
        &self,
        method: String,
        params: Value,
    ) -> Result<Value, InternalError> {
         let (tx, rx) = oneshot::channel();
         let command = SendCommand {
             session_id: Some(self.session_id.clone()), // Use this page's session
             method,
             params,
             result_tx: tx,
         };

         self.command_actor
            .send(command)
            .await
            .map_err(|mb_err| InternalError::Actor(format!("CommandActor mailbox error: {}", mb_err)))??; // Handle both errors


         rx.await.map_err(|_canceled| {
             InternalError::Actor("Command result channel cancelled".to_string())
         })?
    }

    // Helper to subscribe to page-specific events
    fn subscribe_to_page_event(&self, event_name: &str, addr: Recipient<ProtocolEvent>) {
        debug!("PageActor {} subscribing to {}", self.target_id, event_name);
         if self.event_actor.do_send(Subscribe {
             event_name: event_name.to_string(),
             session_id: Some(self.session_id.clone()), // Session-specific subscription
             subscriber: addr,
         }).is_err() {
              error!("Failed to send Subscribe message to EventActor for page {}.", self.target_id);
          }
    }
}

impl Actor for ChromePageActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        info!(
            "ChromePageActor started for target {}, session {}.",
            self.target_id, self.session_id
        );
        self.state = PageActorState::Idle; // Assume idle after start

        // Subscribe to relevant events for this page
        let self_recipient = ctx.address().recipient();
        self.subscribe_to_page_event("Page.lifecycleEvent", self_recipient.clone());
        self.subscribe_to_page_event("Runtime.consoleAPICalled", self_recipient.clone());
        // Add more subscriptions later (DOM.*, Network.*)
    }

    fn stopping(&mut self, _ctx: &mut Context<Self>) -> Running {
        info!(
            "ChromePageActor stopping for target {}, session {}.",
            self.target_id, self.session_id
        );
        self.state = PageActorState::Closed;
        // TODO: Unsubscribe?
        Running::Stop
    }
}

// --- Page Actor Message Handlers ---

impl Handler<Navigate> for ChromePageActor {
    type Result = ResponseFuture<Result<(), InternalError>>;

    fn handle(&mut self, msg: Navigate, _ctx: &mut Context<Self>) -> Self::Result {
        self.state = PageActorState::Navigating; // Update state
        let params = NavigateParams { url: &msg.url };
        let future = self.send_page_command(
            "Page.navigate".to_string(),
            serde_json::to_value(params).unwrap(), // Handle serde error better later
        );

        Box::pin(async move {
            let result = future.await;
            // TODO: Update state based on result / lifecycle events
            // self.state = PageActorState::Idle; // Simplistic update for now
            result?; // Propagate error
            Ok(())
        })
    }
}

impl Handler<EvaluateScript> for ChromePageActor {
    type Result = ResponseFuture<Result<Value, InternalError>>;

    fn handle(&mut self, msg: EvaluateScript, _ctx: &mut Context<Self>) -> Self::Result {
         self.state = PageActorState::Evaluating;
         let params = EvaluateParams {
             expression: &msg.script,
             context_id: None,
             return_by_value: Some(true), // Attempt to get simple values directly
             await_promise: Some(true),   // Await promises by default
         };
        let future = self.send_page_command(
            "Runtime.evaluate".to_string(),
            serde_json::to_value(params).unwrap(),
        );

        Box::pin(async move {
            // self.state = PageActorState::Idle; // Update state later
            let result_value = future.await?;
            let eval_result: EvaluateResult = serde_json::from_value(result_value)
                 .map_err(|e| InternalError::Deserialization(format!("Failed to parse EvaluateResult: {}", e)))?;

            if let Some(exception_details) = eval_result.exception_details {
                 Err(InternalError::Protocol {
                     code: None, // CDP exceptions don't always have codes here
                     message: format!("Script evaluation failed: {}", exception_details.text),
                     data: Some(serde_json::to_string(&exception_details).unwrap_or_default()),
                 })
             } else {
                 Ok(eval_result.result.value) // Return the evaluated value
             }
        })
    }
}

impl Handler<ClosePage> for ChromePageActor {
     type Result = ResponseFuture<Result<(), InternalError>>;

     fn handle(&mut self, _msg: ClosePage, ctx: &mut Context<Self>) -> Self::Result {
         info!("ClosePage message received for target {}", self.target_id);
         self.state = PageActorState::Closing;

        // Send Target.closeTarget command (Browser-level command)
        let command_actor = self.command_actor.clone();
        let target_id = self.target_id.clone();
        let actor_addr = ctx.address(); // Address to stop self

         Box::pin(async move {
            let params = json!({ "targetId": target_id });
             let (tx, rx) = oneshot::channel();
             let command = SendCommand {
                 session_id: None, // Browser-level command
                 method: "Target.closeTarget".to_string(),
                 params,
                 result_tx: tx,
             };

             if command_actor.send(command).await.is_err() {
                 error!("Failed to send closeTarget command for {}", target_id);
                 // Proceed with stopping actor anyway
             } else {
                 match rx.await {
                     Ok(Ok(close_result)) => {
                          // CDP returns {success: bool}
                          if close_result.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
                             info!("Target.closeTarget successful for {}", target_id);
                          } else {
                             warn!("Target.closeTarget reported failure for {}: {:?}", target_id, close_result);
                          }
                     },
                     Ok(Err(e)) => warn!("Error closing target {}: {}", target_id, e),
                     Err(_) => warn!("closeTarget channel cancelled for {}", target_id),
                 }
             }
             // Stop the actor regardless of command success
             actor_addr.stop();
             Ok(())
         })
     }
}


// Handler for ProtocolEvent messages (forwarded by EventActor)
impl Handler<ProtocolEvent> for ChromePageActor {
    type Result = ();

    fn handle(&mut self, msg: ProtocolEvent, _ctx: &mut Context<Self>) {
        // Ensure the event is for this page's session
        if msg.session_id.as_deref() != Some(&self.session_id) {
            warn!("PageActor {} received event for wrong session: {:?}", self.target_id, msg.session_id);
            return;
        }

        trace!("PageActor {} received event: {:?}", self.target_id, msg);
        match msg.method.as_str() {
            "Page.lifecycleEvent" => {
                // Update state based on lifecycle, e.g., navigation completion
                 if let Some(name) = msg.params.get("name").and_then(|v| v.as_str()) {
                     match name {
                         "load" | "networkIdle" | "DOMContentLoaded" => {
                              if self.state == PageActorState::Navigating {
                                 debug!("Page {} reached state: {}", self.target_id, name);
                                 self.state = PageActorState::Idle;
                              }
                         }
                         _ => {}
                     }
                 }
            }
            "Runtime.consoleAPICalled" => {
                // TODO: Parse and potentially emit L1 ConsoleMessage event
                debug!("Console API called on page {}: {:?}", self.target_id, msg.params);
            }
            _ => {} // Ignore other events for now
        }
    }
}
