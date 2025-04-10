//! The main Supervisor actor for the Janus client instance.

use actix::prelude::*;
use janus_browser_chrome::actors::ChromeBrowserActor; // Import browser actor
use janus_core::{error::InternalError, Config};
use janus_protocol_handler::{CommandActor, EventActor}; // Import core actors
use janus_transport::{
    ConnectParams, ConnectionActor, ConnectionState, ConnectionStatusUpdate, IncomingMessage,
};
use log::{debug, error, info, warn};
use std::collections::HashMap;

// --- Supervisor Messages ---

/// Request to start the core actors (Connection, Command, Event).
#[derive(Message)]
#[rtype(result = "Result<CoreActorsInfo, InternalError>")]
pub struct StartCoreActors(pub ConnectParams);

/// Information about the started core actors.
#[derive(Clone)] // Clone to pass around addresses
pub struct CoreActorsInfo {
    pub connection_actor: Addr<ConnectionActor>,
    pub command_actor: Addr<CommandActor>,
    pub event_actor: Addr<EventActor>,
}

/// Request to start a browser-specific actor (e.g., Chrome).
#[derive(Message)]
#[rtype(result = "Result<Addr<ChromeBrowserActor>, InternalError>")] // Example for Chrome
pub struct StartBrowserActor {
    pub core_actors: CoreActorsInfo,
    // pub browser_type: BrowserType, // Could add enum later
}

/// Message sent by ConnectionActor on graceful stop or unexpected termination.
#[derive(Message)]
#[rtype(result = "()")]
pub struct ConnectionTerminated {
    pub actor_addr: Addr<ConnectionActor>, // Identify which connection
    pub error: Option<janus_transport::TransportError>,
}

/// The top-level supervisor actor responsible for managing core actors
/// (Connection, Command, Event) and browser-specific actors.
pub struct SupervisorActor {
    config: Config,
    // State to hold addresses of supervised actors
    connection_actor: Option<Addr<ConnectionActor>>,
    command_actor: Option<Addr<CommandActor>>,
    event_actor: Option<Addr<EventActor>>,
    browser_actors: HashMap<String, Addr<ChromeBrowserActor>>, // Keyed by URL/ID? For now, just one.
}

impl SupervisorActor {
    pub fn new(config: Config) -> Self {
        info!("SupervisorActor created.");
        SupervisorActor {
            config,
            connection_actor: None,
            command_actor: None,
            event_actor: None,
            browser_actors: HashMap::new(),
        }
    }
}

impl Actor for SupervisorActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("SupervisorActor started. Ready to manage actors.");
    }

    fn stopping(&mut self, _ctx: &mut Context<Self>) -> Running {
        info!("SupervisorActor stopping.");
        // Stop all managed actors gracefully if they are still running
        if let Some(addr) = self.browser_actors.remove("chrome") {
            // Assuming single chrome instance for now
            addr.do_send(janus_browser_chrome::actors::ShutdownBrowser);
        }
        if let Some(addr) = self.command_actor.take() {
            addr.do_send(actix::msgs::StopArbiter); // Or specific stop message if exists
        }
        if let Some(addr) = self.event_actor.take() {
            addr.do_send(actix::msgs::StopArbiter);
        }
        if let Some(addr) = self.connection_actor.take() {
            addr.do_send(actix::msgs::StopArbiter); // ConnectionActor stops itself on disconnect
        }
        Running::Stop
    }
}

// --- Message Handlers ---

impl Handler<StartCoreActors> for SupervisorActor {
    type Result = Result<CoreActorsInfo, InternalError>;

    fn handle(&mut self, msg: StartCoreActors, ctx: &mut Context<Self>) -> Self::Result {
        if self.connection_actor.is_some() {
            warn!("Core actors already started. Ignoring request.");
            // Return existing actor addresses
            return Ok(CoreActorsInfo {
                connection_actor: self.connection_actor.clone().unwrap(),
                command_actor: self.command_actor.clone().unwrap(),
                event_actor: self.event_actor.clone().unwrap(),
            });
        }

        info!("Supervisor starting core actors...");
        let connect_params = msg.0;

        // 1. Start EventActor
        let event_actor = EventActor::default().start();
        self.event_actor = Some(event_actor.clone());
        info!("EventActor started at Addr: {:?}", event_actor);

        // 2. Start CommandActor (needs EventActor recipient)
        let command_actor = CommandActor::new(
            self.config.clone(),
            Addr::recipient(&ConnectionActor::from(ctx.address())), // Temporary, need actual Addr
            event_actor.clone().recipient(),
        );
        // Problem: CommandActor needs Addr<ConnectionActor>, but ConnectionActor needs CommandActor recipient. Circular dependency.

        // Solution: Start CommandActor first, but delay giving it ConnectionActor Addr.
        // Or: ConnectionActor sends IncomingMessage to Supervisor, Supervisor forwards to CommandActor? Less direct.
        // Or: Use Supervisor as intermediary for SendMessage? Adds overhead.
        // Let's start CommandActor without ConnectionActor addr, provide it later.
        // Let ConnectionActor send IncomingMessage to CommandActor Addr directly.

        let command_actor_addr = CommandActor::new(
            self.config.clone(),
            Addr::recipient(&ConnectionActor::from(ctx.address())), // Placeholder Addr - MUST BE UPDATED
            event_actor.clone().recipient(),
        )
        .start();
        self.command_actor = Some(command_actor_addr.clone());
        info!("CommandActor started at Addr: {:?}", command_actor_addr);

        // 3. Start ConnectionActor (needs CommandActor recipient for messages)
        let connection_actor = ConnectionActor::new(
            connect_params.clone(),
            command_actor_addr.clone().recipient::<IncomingMessage>(), // CommandActor handles incoming
            ctx.address().recipient::<ConnectionStatusUpdate>(),       // Supervisor handles status
        )
        .start();
        self.connection_actor = Some(connection_actor.clone());
        info!(
            "ConnectionActor starting for {} at Addr: {:?}",
            connect_params.url, connection_actor
        );

        // TODO: Update CommandActor with the actual ConnectionActor address.
        // Need a message for CommandActor like `SetConnectionActor(Addr<ConnectionActor>)`
        // command_actor_addr.do_send(SetConnectionActor(connection_actor.clone()));

        // Need to wait for ConnectionActor to report Connected state?
        // For Phase 2, assume it connects quickly or CommandActor handles NotConnected state.

        Ok(CoreActorsInfo {
            connection_actor,
            command_actor: command_actor_addr,
            event_actor,
        })
    }
}

impl Handler<StartBrowserActor> for SupervisorActor {
    type Result = Result<Addr<ChromeBrowserActor>, InternalError>;

    fn handle(&mut self, msg: StartBrowserActor, _ctx: &mut Context<Self>) -> Self::Result {
        info!("Supervisor starting ChromeBrowserActor...");
        // Ensure core actors are available (passed in message)
        let core_info = msg.core_actors;

        if self.browser_actors.contains_key("chrome") {
            warn!("ChromeBrowserActor already started.");
            return Ok(self.browser_actors.get("chrome").unwrap().clone());
        }

        let browser_actor = ChromeBrowserActor::new(
            core_info.command_actor,
            core_info.event_actor.recipient(), // Pass recipient
        )
        .start();

        info!("ChromeBrowserActor started at Addr: {:?}", browser_actor);
        self.browser_actors
            .insert("chrome".to_string(), browser_actor.clone()); // Assuming one Chrome for now

        // TODO: Need to wait for BrowserActor to become 'Ready' before returning?
        // BrowserActor needs to signal its readiness back to Supervisor or the caller.
        // For Phase 2, return immediately after starting.

        Ok(browser_actor)
    }
}

// Handler for status updates from ConnectionActor
impl Handler<ConnectionStatusUpdate> for SupervisorActor {
    type Result = ();

    fn handle(&mut self, msg: ConnectionStatusUpdate, _ctx: &mut Context<Self>) {
        info!("Supervisor received ConnectionStatusUpdate: {:?}", msg.0);
        // Forward status updates to relevant actors if needed (e.g., CommandActor)
        if let Some(cmd_actor) = &self.command_actor {
            cmd_actor.do_send(msg.0.clone()); // Forward state update
        }

        // Implement supervision logic based on the state update.
        match msg.0 {
            ConnectionState::Disconnected(Some(ref err)) => {
                warn!("Supervised connection failed: {}", err);
                // TODO: Implement restart/cleanup logic (e.g., stop browser actor)
                self.connection_actor = None; // Assume connection is gone
                                              // Potentially stop dependent actors
                if let Some(addr) = self.browser_actors.remove("chrome") {
                    addr.do_send(janus_browser_chrome::actors::ShutdownBrowser);
                }
                if let Some(addr) = self.command_actor.take() {
                    addr.do_send(actix::msgs::StopArbiter);
                }
                if let Some(addr) = self.event_actor.take() {
                    addr.do_send(actix::msgs::StopArbiter);
                }
            }
            ConnectionState::FailedToStart(ref err) => {
                error!("Supervised connection failed to start: {}", err);
                self.connection_actor = None; // Connection never started
                                              // Cleanup actors that would depend on it
                if let Some(addr) = self.command_actor.take() {
                    addr.do_send(actix::msgs::StopArbiter);
                }
                if let Some(addr) = self.event_actor.take() {
                    addr.do_send(actix::msgs::StopArbiter);
                }
            }
            ConnectionState::Connected => {
                info!("Supervisor noted Connection established.");
                // Maybe signal BrowserActor to proceed if it was waiting?
            }
            _ => {
                debug!("Supervisor handling state: {:?}", msg.0);
            }
        }
    }
}

// Handle graceful termination signals (if needed, e.g. from ConnectionActor stopping)
impl Handler<ConnectionTerminated> for SupervisorActor {
    type Result = ();
    fn handle(&mut self, msg: ConnectionTerminated, _ctx: &mut Context<Self>) {
        info!(
            "Supervisor notified of Connection Terminated. Error: {:?}",
            msg.error
        );
        if self.connection_actor.as_ref() == Some(&msg.actor_addr) {
            self.connection_actor = None;
            // Handle cleanup similar to Disconnected state if error occurred
            if msg.error.is_some() {
                // Stop dependent actors
                if let Some(addr) = self.browser_actors.remove("chrome") {
                    addr.do_send(janus_browser_chrome::actors::ShutdownBrowser);
                }
                if let Some(addr) = self.command_actor.take() {
                    addr.do_send(actix::msgs::StopArbiter);
                }
                if let Some(addr) = self.event_actor.take() {
                    addr.do_send(actix::msgs::StopArbiter);
                }
            }
        }
    }
}

// --- Supervision ---
impl Supervised for SupervisorActor {
    fn restarting(&mut self, _ctx: &mut Self::Context) {
        info!("SupervisorActor restarting...");
        // Clean up state before restart if necessary
        self.connection_actor = None;
        self.command_actor = None;
        self.event_actor = None;
        self.browser_actors.clear();
    }
}

// --- Placeholder handler from Phase 1 (REMOVE or update) ---
// impl Handler<LaunchConnection> for SupervisorActor { ... } - Delete this.
