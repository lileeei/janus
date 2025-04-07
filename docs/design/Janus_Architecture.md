
# Janus Client Architecture Design (Consolidated)

**Note:** This document merges and refines the designs previously described in `ARCHITECTURE.md` and `ACTOR_DESIGN.md`.

## 1. Overview

Janus Client is designed as a unified browser debugging protocol client. It supports multiple browser debugging protocols (like CDP, WebDriver BiDi) through a combination of a layered architecture and an actor model. The system aims to be protocol-agnostic at its top layer, extensible, maintainable, performant, and robust, leveraging Rust's strengths in type safety and asynchronous programming.

## 2. Design Goals

### 2.1 Primary Goals
- **Protocol Agnostic:** Provide a unified L1 API hiding underlying protocol differences.
- **Layered Architecture:** Enforce clear separation of concerns across three distinct layers.
- **Actor-Based Concurrency:** Utilize the actor model (specifically `actix` or a similar framework) for managing concurrency, state isolation, and fault tolerance.
- **Type Safety:** Leverage Rust's type system for compile-time guarantees and safety.
- **Async First:** Built entirely with `async/await` for non-blocking I/O and performance.
- **Robust Error Handling:** Implement comprehensive error handling, reporting, and recovery mechanisms across layers and actors.

### 2.2 Secondary Goals
- **Extensibility:** Easily add support for new browsers, protocols, or features (e.g., plugins).
- **Configuration:** Offer a flexible configuration system.
- **Monitoring:** Include built-in capabilities for monitoring and diagnostics.
- **Testability:** Design for unit, integration, and end-to-end testing.
- **Documentation:** Provide comprehensive documentation for users and developers.
- **Plugin Support:** Allow for extending functionality via a plugin system (potentially with hot-reloading).

## 3. System Architecture

### 3.1 Layered Architecture

The system employs a three-layer architecture:

```
┌─────────────────────────────────────┐
│      Unified Interface Layer (L1)    │  ◄── User Interaction Point
│  - Protocol-agnostic Browser/Page API│
│  - Defines common operations/events  │
│  - Hides implementation complexity   │
│  - Maps internal errors to API errors│
├─────────────────────────────────────┤
│    Browser Implementation Layer (L2) │  ◄── Protocol Logic
│  - Implements L1 traits per browser  │
│  - Contains browser-specific logic   │
│  - Translates L1 calls to protocol   │
│    commands/events (via Actors)      │
│  - Manages browser/page state actors │
├─────────────────────────────────────┤
│     Transport & Connection Layer (L3)│  ◄── Raw Communication
│  - Manages raw network connections   │
│    (WebSocket, TCP, IPC)             │
│  - Handles message serialization/    │
│    deserialization (basic framing)   │
│  - Manages connection lifecycle      │
│  - Managed by ConnectionActor        │
└─────────────────────────────────────┘
```

*   **L1 (Unified Interface):** Provides the public API. Users interact with traits like `Browser` and `Page`. This layer ensures consistency regardless of the underlying browser or protocol.
*   **L2 (Browser Implementation):** Contains the specific logic for each supported browser (Chrome, Firefox, Edge). It implements the L1 traits and orchestrates communication with the browser using the appropriate protocol (CDP, BiDi, etc.). This is where protocol-specific commands and events are handled, primarily within Actors specific to this layer.
*   **L3 (Transport & Connection):** Deals with the low-level details of establishing and maintaining a connection (e.g., WebSocket handshake, message framing) and sending/receiving raw byte streams or messages.

### 3.2 Actor System Integration

The actor model is used *within* these layers to manage concurrency, state, and communication flow. It is an *implementation detail* hidden by L1.

```
            ┌──────────────────────────┐
            │     Supervisor Actor     │ Manages lifecycle & errors
            ├──────────────────────────┤
 L2 Actors: │ ┌────────────┐ ┌─────────┐│ ┌────────────┐
            │ │Browser Actor│ │Page Actor││ │Plugin Actor│
            │ └────────────┘ └─────────┘│ └────────────┘
            ├──────────────────────────┤
Core Actors:│ ┌─────────────┐┌─────────┐│ ┌────────────┐
            │ │Command Actor││Event Actor││ │Monitor Actor│
            │ └─────────────┘└─────────┘│ └────────────┘
            ├──────────────────────────┤
 L3 Actor:  │   ┌──────────────────┐   │
            │   │ Connection Actor │   │ Manages raw transport
            │   └──────────────────┘   │
            └──────────────────────────┘
```

### 3.3 Directory Structure (Conceptual)

```
src/
├── core/               # Core components (potentially Actors like Supervisor, Event, Command, Monitor)
│   ├── actor/          # Actor system setup, common messages, traits
│   ├── error/          # Common error types
│   └── config/         # Configuration loading and structures
├── interface/          # L1 Unified Interfaces (Browser, Page traits, API errors)
├── browsers/           # L2 Browser Implementations
│   ├── chrome/         # Chrome (CDP) implementation
│   │   ├── actors.rs   # BrowserActor, PageActor (Chrome specific)
│   │   ├── protocol.rs # CDP specific command/event mapping
│   │   └── mod.rs      # ChromeBrowser struct implementing L1 traits
│   ├── firefox/        # Firefox (WebDriver BiDi / Remote Protocol) implementation
│   └── edge/           # Edge (CDP) implementation
└── transport/          # L3 Transport Implementations
    ├── connection.rs   # Connection trait, ConnectionActor definition
    ├── websocket.rs    # WebSocket transport logic
    ├── tcp.rs          # TCP transport logic
    └── ipc.rs          # IPC transport logic
```

## 4. Core Components

### 4.1 Layer Components

#### 4.1.1 L1 - Unified Interface Layer

Provides a stable, protocol-agnostic API.

```rust
// src/interface/mod.rs (or similar)
use serde_json::Value; // Example type

// Represents common errors surfaced to the user
pub enum ApiError {
    ConnectionFailed(String),
    Timeout,
    ProtocolError(String),
    BrowserCrashed,
    InvalidParameters(String),
    // ... other high-level errors
}

pub trait Browser {
    // Lifecycle & Connection
    async fn connect(&mut self) -> Result<(), ApiError>;
    async fn disconnect(&mut self) -> Result<(), ApiError>;
    async fn close(&mut self) -> Result<(), ApiError>; // Close the browser process

    // Page Management
    async fn new_page(&self) -> Result<Box<dyn Page>, ApiError>;
    async fn pages(&self) -> Result<Vec<Box<dyn Page>>, ApiError>; // Get handles to existing pages

    // Browser-level operations
    async fn version(&self) -> Result<String, ApiError>;

    // Event Subscription (Example)
    // async fn on_target_created(&self, handler: Box<dyn Fn(Box<dyn Page>) + Send + Sync>) -> Result<SubscriptionId, ApiError>;
    // async fn unsubscribe(&self, id: SubscriptionId) -> Result<(), ApiError>;
}

pub trait Page {
    // Navigation
    async fn navigate(&self, url: &str) -> Result<(), ApiError>;
    async fn reload(&self) -> Result<(), ApiError>;
    async fn go_back(&self) -> Result<(), ApiError>;
    async fn go_forward(&self) -> Result<(), ApiError>;

    // Lifecycle
    async fn close(&self) -> Result<(), ApiError>;
    fn id(&self) -> String; // Get the page/target identifier

    // Content & Scripting
    async fn content(&self) -> Result<String, ApiError>; // Get HTML content
    async fn evaluate_script(&self, script: &str) -> Result<Value, ApiError>;
    async fn call_function(&self, function_declaration: &str, args: Vec<Value>) -> Result<Value, ApiError>;

    // DOM Interaction
    async fn query_selector(&self, selector: &str) -> Result<Option<ElementHandle>, ApiError>; // ElementHandle would be another L1 abstraction
    async fn wait_for_selector(&self, selector: &str, timeout_ms: u64) -> Result<ElementHandle, ApiError>;

    // Input
    // async fn click(&self, selector: &str) -> Result<(), ApiError>;
    // async fn type_text(&self, selector: &str, text: &str) -> Result<(), ApiError>;

    // Information
    async fn url(&self) -> Result<String, ApiError>;
    async fn title(&self) -> Result<String, ApiError>;

    // Screenshot
    async fn take_screenshot(&self, format: ScreenshotFormat, options: ScreenshotOptions) -> Result<Vec<u8>, ApiError>;

    // Event Subscription (Page-level)
    // async fn on_load(&self, handler: Box<dyn Fn() + Send + Sync>) -> Result<SubscriptionId, ApiError>;
    // async fn on_console_message(&self, handler: Box<dyn Fn(ConsoleMessage) + Send + Sync>) -> Result<SubscriptionId, ApiError>;
}

// Other potential L1 types: ElementHandle, ConsoleMessage, ScreenshotFormat, ScreenshotOptions, SubscriptionId etc.
```

#### 4.1.2 L2 - Browser Implementation Layer

Implements L1 traits using specific browser protocols and actors.

```rust
// src/browsers/chrome/mod.rs (Example)
use crate::interface::{Browser, Page, ApiError};
use crate::core::actor::{Addr, System}; // Assuming Addr, System from actor framework
use super::actors::{ChromeBrowserActor, ChromePageActor, SupervisorMsg, BrowserCtrlMsg, PageCtrlMsg};
use crate::transport::ConnectParams; // Parameters for connection

pub struct ChromeBrowser {
    // Internal handle to the actor managing this browser instance.
    // NOT exposed directly to the user.
    actor_addr: Addr<ChromeBrowserActor>,
    // Maybe actor system handle if needed for spawning pages? Or managed by Supervisor.
    // system: System,
}

impl ChromeBrowser {
    pub async fn launch(params: ConnectParams /* , config: &Config */) -> Result<Self, ApiError> {
        // 1. Start the Actor System (if not already running globally/per-instance)
        // 2. Start the Supervisor Actor
        // 3. Ask Supervisor to launch ChromeBrowserActor (which might launch ConnectionActor etc.)
        // 4. Wait for BrowserActor to signal readiness
        // Example (pseudo-code):
        // let supervisor = SupervisorActor::start_default(); // Or get handle
        // let actor_addr = supervisor.send(SupervisorMsg::LaunchBrowser(BrowserType::Chrome, params)).await??;
        // Ok(Self { actor_addr })
        todo!("Implement browser launching and actor setup")
    }
}

impl Browser for ChromeBrowser {
    async fn navigate(&self, url: &str) -> Result<(), ApiError> {
        // 1. Construct the appropriate protocol command (CDP's Page.navigate)
        // 2. Create a message for the PageActor (requires getting the right PageActor Addr, maybe via BrowserActor?)
        // 3. Send the message (e.g., using actor_addr.send().await)
        // 4. Handle the actor's response (Result<_, ActorError>)
        // 5. Map ActorError/ProtocolError to ApiError
        // Example (pseudo-code):
        // self.actor_addr.send(BrowserCtrlMsg::NavigatePage { page_id: /* somehow get active/default page id */, url: url.to_string() })
        //     .await // Request-response pattern
        //     .map_err(|actor_err| ApiError::ProtocolError(actor_err.to_string()))? // Handle mailbox errors
        //     .map_err(|protocol_err| ApiError::ProtocolError(protocol_err.to_string())) // Handle protocol errors from actor response
        todo!("Implement navigate by sending message to appropriate actor")
    }

    async fn new_page(&self) -> Result<Box<dyn Page>, ApiError> {
        // 1. Send message to BrowserActor to create a new target/page
        // 2. BrowserActor interacts with browser via CommandActor/ConnectionActor (e.g., Target.createTarget)
        // 3. BrowserActor receives new target ID, spawns a new ChromePageActor
        // 4. BrowserActor returns the Addr<ChromePageActor>
        // 5. Wrap the Addr in a ChromePage struct
        // 6. Return Box::new(ChromePage { ... })
        todo!()
    }

    // ... other Browser trait methods implemented similarly ...
}

// Represents a handle to a specific Chrome page/target
pub struct ChromePage {
    actor_addr: Addr<ChromePageActor>,
    page_id: String,
}

impl Page for ChromePage {
     async fn close(&self) -> Result<(), ApiError> {
         // Send Close message to self.actor_addr
         // Actor handles Target.closeTarget command
         todo!()
     }
     fn id(&self) -> String {
         self.page_id.clone()
     }
     // ... other Page trait methods implemented by sending messages to self.actor_addr ...
}

```

#### 4.1.3 L3 - Transport & Connection Layer

Manages the raw connection and message transport.

```rust
// src/transport/connection.rs
use async_trait::async_trait;
use tokio::sync::mpsc; // Example channel type
use crate::core::error::TransportError;

#[async_trait]
pub trait Transport {
    async fn connect(&mut self) -> Result<(), TransportError>;
    async fn disconnect(&mut self) -> Result<(), TransportError>;
    async fn send(&self, message: String) -> Result<(), TransportError>;
    async fn receive(&mut self) -> Option<Result<String, TransportError>>; // Stream-like receive
}

// Parameters needed to establish a connection
pub struct ConnectParams {
    pub url: String, // e.g., ws://localhost:9222/devtools/browser/...
    pub timeout_ms: u64,
    // ... other options like headers, proxy etc.
}

// Actor responsible for managing a single connection
use actix::prelude::*;
pub struct ConnectionActor {
    transport: Option<Box<dyn Transport + Unpin + Send>>, // The underlying WebSocket/TCP/IPC transport
    params: ConnectParams,
    state: ConnectionState,
    // Actor to forward incoming messages to (e.g., EventActor or CommandActor)
    message_handler: Recipient<IncomingMessage>,
    // Supervisor or parent actor for reporting critical errors
    // supervisor: Addr<SupervisorActor>,
}

#[derive(Message)]
#[rtype(result = "Result<(), TransportError>")]
pub struct SendMessage(pub String); // Message to send out

#[derive(Message)]
#[rtype(result = "()")]
pub struct IncomingMessage(pub String); // Message received from transport

pub enum ConnectionState { Idle, Connecting, Connected, Disconnecting, Disconnected(Option<TransportError>)}

impl Actor for ConnectionActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        log::info!("ConnectionActor starting for {}", self.params.url);
        self.state = ConnectionState::Connecting;
        // Spawn a task to handle the actual connection and read loop
        let transport_builder = create_transport(&self.params); // Factory function
        let addr = ctx.address();
        let message_handler = self.message_handler.clone();

        ctx.spawn(async move {
            match transport_builder.connect().await {
                Ok(mut transport) => {
                    addr.do_send(ConnectionStatusUpdate(ConnectionState::Connected));
                    while let Some(msg_result) = transport.receive().await {
                        match msg_result {
                            Ok(msg) => message_handler.do_send(IncomingMessage(msg)),
                            Err(e) => {
                                log::error!("Transport receive error: {}", e);
                                addr.do_send(ConnectionStatusUpdate(ConnectionState::Disconnected(Some(e))));
                                break; // Exit read loop on error
                            }
                        }
                    }
                    // If loop exits without error, it means connection closed gracefully
                    log::info!("Transport disconnected gracefully.");
                    addr.do_send(ConnectionStatusUpdate(ConnectionState::Disconnected(None)));

                },
                Err(e) => {
                    log::error!("Transport connect error: {}", e);
                    addr.do_send(ConnectionStatusUpdate(ConnectionState::Disconnected(Some(e))));
                }
            }
        }.into_actor(self));
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct ConnectionStatusUpdate(ConnectionState);

impl Handler<ConnectionStatusUpdate> for ConnectionActor {
    type Result = ();
    fn handle(&mut self, msg: ConnectionStatusUpdate, _ctx: &mut Context<Self>) {
        log::info!("Connection state updated: {:?}", msg.0);
        self.state = msg.0;
        // Notify supervisor or interested parties about state change
    }
}


impl Handler<SendMessage> for ConnectionActor {
    type Result = ResponseFuture<Result<(), TransportError>>;

    fn handle(&mut self, msg: SendMessage, _ctx: &mut Context<Self>) -> Self::Result {
        // TODO: Handle state check (must be connected)
        // Need to access transport - this design needs refinement.
        // Maybe transport is managed directly in the async block or passed via message?
        // Or store transport in the actor state once connected.
        // For simplicity, assume self.transport is Some(connected_transport)
        Box::pin(async move {
            // if let Some(transport) = &self.transport {
            //     transport.send(msg.0).await
            // } else {
            //     Err(TransportError::NotConnected)
            // }
            todo!("Implement sending via transport")
        })
    }
}

// Helper function (outside actor)
fn create_transport(params: &ConnectParams) -> Box<dyn Transport + Unpin + Send> {
    // Logic to choose WebSocket, TCP etc. based on params.url scheme
    #[cfg(feature = "websocket")]
    if params.url.starts_with("ws://") || params.url.starts_with("wss://") {
         return Box::new(crate::transport::websocket::WebSocketTransport::new(params));
    }
    // ... other transport types
    panic!("Unsupported transport scheme in URL: {}", params.url);
}

```

### 4.2 Actor System Design

#### 4.2.1 Overview and Rationale

The actor model provides:
*   **Concurrency:** Handles multiple simultaneous operations (e.g., commands, event processing) without manual lock management.
*   **State Encapsulation:** Each actor manages its own state, preventing race conditions.
*   **Asynchronous Communication:** Messages are sent asynchronously, fitting well with the `async/await` paradigm.
*   **Fault Isolation:** Errors in one actor can be contained and managed by its supervisor without necessarily crashing the entire application.

#### 4.2.2 Key Actors and Responsibilities

*   **`Supervisor Actor` (Core):**
    *   Responsibilities: Top-level management, launching and overseeing core actors (`BrowserActor`, `CommandActor`, `EventActor`, `MonitorActor`), defining supervision strategies (restart, stop, escalate on child failure), potentially managing global resources or configuration.
    *   State: Addresses of child actors it supervises.
*   **`Browser Actor` (L2 - Per Browser Instance):**
    *   Responsibilities: Manages the lifecycle of a specific browser *instance* (if applicable, e.g., launching the process), handles browser-level protocol commands (e.g., `Browser.getVersion`, `Target.createTarget`), manages associated `PageActor`s, maintains overall browser state (`Ready`, `Crashed`). Communicates with `CommandActor` to send commands and receives relevant events via `EventActor`.
    *   State: `BrowserState`, Map of `TargetId -> Addr<PageActor>`, potentially browser process info.
*   **`Page Actor` (L2 - Per Browser Tab/Target):**
    *   Responsibilities: Manages the state and operations for a single browser page or target. Handles page-specific protocol commands (e.g., `Page.navigate`, `Runtime.evaluate`, `DOM.querySelector`) by sending messages to `CommandActor`. Subscribes to relevant events (e.g., `Page.loadEventFired`, `Runtime.consoleAPICalled`) via `EventActor`. Implements the logic backing the L1 `Page` trait methods.
    *   State: `PageState`, `TargetId`/`SessionId`, URL, potentially cached DOM info.
*   **`Connection Actor` (L3 - Per Connection):**
    *   Responsibilities: Manages a *single* underlying transport connection (WebSocket, TCP, etc.). Handles connecting, disconnecting, sending raw protocol messages, receiving raw messages, basic framing/parsing (if needed by transport), and reporting transport-level errors. Forwards successfully received and framed messages to a designated handler (likely `CommandActor` for responses, `EventActor` for events, or a combined parser).
    *   State: `ConnectionState`, underlying `Transport` object, address of message handler.
*   **`Command Actor` (Core/L2 - Singleton or Per Browser):**
    *   Responsibilities: **Orchestrates command execution.** Receives abstract command requests (from `BrowserActor`/`PageActor`), translates them into the specific wire protocol format (e.g., JSON-RPC for CDP), assigns unique message IDs, sends the formatted message via the `ConnectionActor`, tracks pending requests (maps message ID to waiting actor/future), receives responses (likely routed from `ConnectionActor`), matches responses to pending requests by ID, handles command timeouts, and sends the result (or error) back to the original requester.
    *   State: Map of `RequestId -> PendingRequestInfo (reply_to, timeout_timer)`, `RequestId` counter, potentially `Addr<ConnectionActor>`.
*   **`Event Actor` (Core - Singleton or Per Browser):**
    *   Responsibilities: Receives all incoming protocol events (likely routed from `ConnectionActor` after basic parsing), parses/categorizes events based on their type/domain (e.g., `Page.loadEventFired`, `Network.requestWillBeSent`), manages event subscriptions from various actors (`BrowserActor`, `PageActor`, `PluginActor`, L1 client handlers), and distributes events to all registered subscribers for that event type.
    *   State: Map of `EventType -> Vec<SubscriberAddress>`, potentially an event buffer.
*   **`Plugin Actor` (L2 - Per Plugin Instance):**
    *   Responsibilities: Manages the lifecycle and execution of a single plugin. Interacts with other actors (`CommandActor`, `EventActor`) to observe or control the browser based on plugin logic.
    *   State: Plugin-specific state, `PluginId`.
*   **`Monitor Actor` (Core - Optional Singleton):**
    *   Responsibilities: Gathers metrics (actor mailbox sizes, message throughput, command latency), monitors actor health (e.g., via heartbeats or supervision), logs system-level diagnostic information.
    *   State: Metrics data, health status of monitored actors.

#### 4.2.3 Message Flow

*   **Command Execution:**
    1.  `Client (User Code)` calls `L1 Interface` method (e.g., `page.navigate("url")`).
    2.  `L2 Implementation` (e.g., `ChromePage::navigate`) translates the call into an internal message (e.g., `ExecutePageCommand { method: "Page.navigate", params: ... }`).
    3.  `L2 Implementation` sends the message to its corresponding `Page Actor`.
    4.  `Page Actor` receives the message, potentially performs validation, then sends a structured command message (e.g., `SendCommand { target_id, method, params, reply_to: self }`) to the `Command Actor`.
    5.  `Command Actor` generates a unique request ID, translates the command to the wire format (JSON-RPC), stores the `reply_to` address and request ID, and sends the raw message string via a `SendMessage` message to the `Connection Actor`.
    6.  `Connection Actor` sends the raw message over the transport (e.g., WebSocket).
*   **Response Handling:**
    1.  `Browser` sends a response message over the transport.
    2.  `Connection Actor` receives the raw message.
    3.  `Connection Actor` parses it minimally (e.g., decodes JSON) and determines if it's a response (has an `id`) or an event.
    4.  If it's a response, `Connection Actor` sends the parsed message (e.g., `IncomingMessage(json_value)`) to the `Command Actor` (as configured).
    5.  `Command Actor` extracts the request ID, finds the corresponding `PendingRequestInfo`, cancels the timeout timer, and sends the response payload (or error) back to the stored `reply_to` address (the original `Page Actor`).
    6.  `Page Actor` receives the result, processes it, and resolves the future/callback associated with the initial L2 implementation call.
    7.  `L2 Implementation` maps the internal result/error to the L1 `Result<_, ApiError>` and returns it to the `Client`.
*   **Event Handling:**
    1.  `Browser` sends an event message (no `id` field) over the transport.
    2.  `Connection Actor` receives the raw message.
    3.  `Connection Actor` parses it minimally and determines it's an event.
    4.  `Connection Actor` sends the parsed message (e.g., `IncomingMessage(json_value)`) to the `Event Actor` (as configured).
    5.  `Event Actor` fully parses the event, determines its type/domain (e.g., `Page.loadEventFired`), looks up subscribers for that event type.
    6.  `Event Actor` sends the parsed event message to each subscribed actor (`BrowserActor`, `PageActor`, `PluginActor`, L1 Handler Adapters).
    7.  Subscribed actors handle the event according to their logic.

#### 4.2.4 Actor State Management

*   **Generic Actor State:** Actors may internally track lifecycle state: `Starting`, `Running`, `Stopping`, `Failed(Error)`. Supervisors use this.
*   **Domain-Specific State:** Key actors manage state relevant to their domain:
    *   `ConnectionActor`: `ConnectionState` (Idle, Connecting, Connected, Disconnecting, Disconnected).
    *   `BrowserActor`: `BrowserState` (Initializing, Ready, Degraded, Crashed, Closed).
    *   `PageActor`: `PageState` (Loading, Interactive, Complete, Crashed, Closed), Current URL, Target ID.
*   State transitions often trigger notifications or internal logic changes (e.g., `ConnectionState::Connected` enables message sending).

#### 4.2.5 Actor Error Handling & Supervision

*   **Actor Internal Errors:** Handled within the actor's message handlers, potentially returning `Err` in `Result` for request-response messages.
*   **Transport Errors:** Handled primarily by `ConnectionActor`. May attempt retries or transition to `Disconnected` state, notifying supervisor.
*   **Protocol Errors:** Handled by `CommandActor` (e.g., malformed responses, browser error responses) or `EventActor` (e.g., failed event parsing). Propagated back to the requesting actor or logged.
*   **Actor Panics / Critical Errors:** Caught by the actor framework and reported to the `Supervisor Actor`.
*   **Supervision Strategy:** The `Supervisor Actor` defines how to react to child actor failures (based on the actor and error type):
    *   `SupervisorAction::Restart`: Attempt to restart the failed actor. Useful for recoverable errors (e.g., temporary connection loss if `ConnectionActor` fails).
    *   `SupervisorAction::Stop`: Stop the failed actor. Appropriate if the error is fatal for that actor's function.
    *   `SupervisorAction::Escalate`: Propagate the error up the supervision hierarchy (if nested).
    *   `SupervisorAction::Resume`: Ignore the error and let the actor continue (use with caution).
*   **Propagation to L1:** Critical unrecoverable errors managed by the supervisor (e.g., browser crash detected by `BrowserActor`, persistent connection failure) should eventually result in methods on the L1 `Browser` or `Page` traits returning an appropriate `ApiError` (e.g., `ApiError::BrowserCrashed`, `ApiError::ConnectionFailed`).

### 4.3 Configuration System

Configuration can be loaded from files (e.g., TOML, YAML) or environment variables.

```rust
// src/core/config.rs
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default)]
    pub global: GlobalConfig,
    #[serde(default)]
    pub browser_defaults: BrowserDefaults,
    #[serde(default)]
    pub browsers: HashMap<String, BrowserSpecificConfig>, // e.g., "chrome", "firefox"
    #[serde(default)]
    pub transport: TransportConfig,
    #[serde(default)]
    pub actor_system: ActorSystemConfig,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct GlobalConfig {
    pub log_level: String, // e.g., "info", "debug"
    pub default_command_timeout_ms: u64,
}

impl Default for GlobalConfig { /* ... */ }

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct BrowserDefaults {
    pub user_data_dir_base: Option<String>,
    pub headless: bool,
    pub args: Vec<String>, // Default args for all browsers
}

impl Default for BrowserDefaults { /* ... */ }

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct BrowserSpecificConfig {
    pub executable_path: Option<String>,
    pub user_data_dir: Option<String>, // Overrides default base
    pub args: Option<Vec<String>>, // Overrides default args
    pub protocol_port: Option<u16>, // e.g., CDP port
    // Other browser-specific settings
}

impl Default for BrowserSpecificConfig { /* ... */ }


#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct TransportConfig {
    pub connect_timeout_ms: u64,
    pub websocket: WebSocketConfig,
    // pub tcp: TcpConfig,
}

impl Default for TransportConfig { /* ... */ }


#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct WebSocketConfig {
    pub max_message_size: Option<usize>,
    pub accept_unmasked_frames: bool, // For some older protocols/proxies
}

impl Default for WebSocketConfig { /* ... */ }


#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct ActorSystemConfig {
    pub default_mailbox_capacity: usize,
    // Potentially thread pool sizes, supervision strategy defaults etc.
}

impl Default for ActorSystemConfig { /* ... */ }

```

## 5. Implementation Plan

### Phase 1: Foundation
- [ ] Setup project structure, basic dependencies (`tokio`, `actix`/`ractor`, `serde`, `log`).
- [ ] Define core L1 traits (`Browser`, `Page`) and `ApiError`.
- [ ] Implement basic actor system setup (`SupervisorActor` skeleton).
- [ ] Define core actor message types (Command/Event wrappers).
- [ ] Implement configuration loading (`Config` struct).
- [ ] Implement basic L3 `Transport` trait and `ConnectionActor` structure.
- [ ] Implement WebSocket `Transport`.

### Phase 2: Core Actor Logic & Chrome CDP
- [ ] Implement `CommandActor` logic (ID generation, pending requests, basic translation).
- [ ] Implement `EventActor` logic (subscription management, event distribution).
- [ ] Implement `ConnectionActor` logic (connection task, message routing to Command/Event actors).
- [ ] Implement L2 `ChromeBrowserActor` and `ChromePageActor` skeletons.
- [ ] Implement L2 `ChromeBrowser` struct implementing basic `Browser` methods (e.g., `connect`, `disconnect`, `version`, `new_page`).
- [ ] Implement L2 `ChromePage` struct implementing basic `Page` methods (e.g., `navigate`, `evaluate_script`, `close`). Requires interaction with `CommandActor`.
- [ ] Connect L2 actors to `CommandActor` and `EventActor`.

### Phase 3: Features & Refinement
- [ ] Implement robust error handling and supervision strategies.
- [ ] Implement full L1 `Browser` and `Page` interfaces for Chrome.
- [ ] Implement event handling from browser -> L1 client.
- [ ] Implement session/target management within `BrowserActor`.
- [ ] Develop `MonitorActor` for basic metrics.
- [ ] Add comprehensive unit and integration tests for actors and layers.

### Phase 4: Additional Browsers & Polish
- [ ] Add Firefox support (WebDriver BiDi or Marionette) - requires new L2 implementation and potentially protocol adaptation in `CommandActor`/`EventActor`.
- [ ] Add Edge support (likely reusing much of Chrome's CDP implementation).
- [ ] Implement Plugin system (`PluginActor`).
- [ ] Add End-to-End tests against real browsers.
- [ ] Performance optimization and benchmarking.
- [ ] Documentation refinement.

## 6. Testing Strategy

*   **Unit Testing:**
    *   Test individual actor message handling logic using mock messages and state assertions.
    *   Test L2 protocol translation functions.
    *   Test configuration parsing.
    *   Test L3 transport message framing/parsing (if applicable).
    *   Use `actix::test` or equivalent for actor tests.
*   **Integration Testing:**
    *   Test interaction flows between actors (`PageActor` -> `CommandActor` -> `ConnectionActor`).
    *   Test `EventActor` subscription and dispatch.
    *   Test `SupervisorActor` strategies by inducing failures.
    *   Test L1 interface calls through the L2 implementation to actors (mocking the `ConnectionActor`'s browser interaction).
*   **End-to-End Testing:**
    *   Run tests that launch a real browser instance.
    *   Execute common L1 API operations (navigation, scripting, DOM interaction, events).
    *   Verify expected browser state and results.
    *   Test against multiple supported browsers (Chrome, Firefox, Edge).
    *   Include stress tests and basic performance benchmarks.

## 7. Documentation

*   **User Documentation:** Getting Started guide, Configuration options, L1 API reference (`Browser`, `Page` traits), Browser-specific setup/notes, Examples.
*   **Developer Documentation:** This Architecture document, Implementation details for each layer/actor, Protocol handling specifics (CDP/BiDi mapping), Contribution guidelines, Testing strategy.
*   **API Documentation:** Auto-generated Rust documentation (`cargo doc`) for all public modules, traits, structs, and functions, including L1 interfaces and configuration structs.

## 8. Future Considerations

*   **Performance:** Actor mailbox optimization, message batching (if applicable), connection pooling (for WebDriver scenarios?), zero-copy parsing where possible.
*   **Security:** Handling authentication (e.g., WebDriver capabilities), secure connection options (WSS, TLS), sandboxing for plugins.
*   **Extensibility:** Formalizing the plugin API, supporting custom transport protocols, framework for adding new browser/protocol support more easily.
*   **Advanced Features:** Network interception/mocking, advanced debugging features (breakpoints, stepping), coverage data collection.

## 9. Version Roadmap (Tentative)

*   **v0.1.0 - Foundation:** Phase 1 complete. Basic connection and command execution for Chrome.
*   **v0.2.0 - Chrome Complete:** Phase 2 complete. Functional Chrome CDP implementation covering most L1 APIs. Basic event handling.
*   **v0.3.0 - Robustness & Features:** Phase 3 complete. Stable Chrome support, error handling, basic monitoring, testing improvements.
*   **v0.4.0 - Firefox Support:** Phase 4 progress. Add support for Firefox via WebDriver BiDi (or alternative).
*   **v0.5.0 - Edge & Plugins:** Phase 4 progress. Add Edge support, implement initial plugin system.
*   **v1.0.0 - Production Ready:** All phases complete. Full browser support, extensive testing, performance tuning, complete documentation.
