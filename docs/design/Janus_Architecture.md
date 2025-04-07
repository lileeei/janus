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

### 3.3 Directory Structure (Workspace Crates by Functionality)

To promote modularity, maintainability, and clearer dependency management, the Janus Client project is structured as a Rust workspace. Each crate within the workspace encapsulates a specific area of functionality, aligning with the layered architecture but providing more concrete separation.

```
janus-client-workspace/
├── Cargo.toml            # Defines the workspace and members
├── crates/
│   ├── janus-interfaces/ # L1 - Public API Contract & Core Types
│   │   ├── src/
│   │   │   ├── browser.rs  # Browser trait definition
│   │   │   ├── page.rs     # Page trait definition
│   │   │   ├── common.rs   # Common types (ElementHandle, etc.)
│   │   │   ├── error.rs    # ApiError enum definition
│   │   │   └── lib.rs
│   │   └── Cargo.toml      # Minimal dependencies (maybe serde)
│   │
│   ├── janus-core/       # Shared Utilities & Core Services
│   │   ├── src/
│   │   │   ├── config.rs   # Configuration loading/structs (Section 4.3)
│   │   │   ├── error.rs    # Internal error types (distinct from ApiError)
│   │   │   ├── actor/      # Base actor traits, common messages, Supervisor concept?
│   │   │   └── lib.rs
│   │   └── Cargo.toml      # Dependencies: serde, config, log, actor framework
│   │
│   ├── janus-transport/  # L3 - Transport Abstraction & Implementation
│   │   ├── src/
│   │   │   ├── connection.rs # ConnectionActor definition
│   │   │   ├── traits.rs     # Transport trait definition
│   │   │   ├── websocket.rs  # WebSocket transport implementation
│   │   │   ├── tcp.rs        # (Optional) TCP transport implementation
│   │   │   ├── ipc.rs        # (Optional) IPC transport implementation
│   │   │   └── lib.rs
│   │   └── Cargo.toml      # Dependencies: janus-core (errors), tokio, async-trait, actor framework, websocket libs
│   │                     # Features: ["websocket", "tcp", "ipc"]
│   │
│   ├── janus-protocol-handler/ # Core Protocol Interaction Logic (Command/Event Routing)
│   │   ├── src/
│   │   │   ├── command_actor.rs # CommandActor implementation (ID tracking, routing)
│   │   │   ├── event_actor.rs   # EventActor implementation (subscription, dispatch)
│   │   │   ├── messages.rs    # Messages specific to Command/Event actors
│   │   │   └── lib.rs
│   │   └── Cargo.toml      # Dependencies: janus-core, janus-interfaces(errors?), janus-transport (for Addr<ConnectionActor>), actor framework, serde_json
│   │
│   ├── janus-browser-chrome/ # L2 - Chrome (CDP) Implementation
│   │   ├── src/
│   │   │   ├── browser.rs    # ChromeBrowser struct implementing janus_interfaces::Browser
│   │   │   ├── page.rs       # ChromePage struct implementing janus_interfaces::Page
│   │   │   ├── actors.rs     # ChromeBrowserActor, ChromePageActor definitions
│   │   │   ├── protocol.rs   # CDP command/event serialization/deserialization helpers
│   │   │   └── lib.rs
│   │   └── Cargo.toml      # Dependencies: janus-interfaces, janus-core, janus-protocol-handler, actor framework, serde, serde_json
│   │
│   ├── janus-browser-firefox/ # L2 - Firefox (BiDi/Marionette) Implementation
│   │   ├── src/            # Similar structure to janus-browser-chrome
│   │   │   ├── browser.rs
│   │   │   ├── page.rs
│   │   │   ├── actors.rs
│   │   │   ├── protocol.rs   # BiDi/Marionette protocol helpers
│   │   │   └── lib.rs
│   │   └── Cargo.toml      # Dependencies: janus-interfaces, janus-core, janus-protocol-handler, actor framework, serde, serde_json
│   │
│   ├── janus-client/     # Main Library Crate (User Entry Point)
│   │   ├── src/
│   │   │   ├── launch.rs   # Functions like launch_chrome(), launch_firefox()
│   │   │   ├── lib.rs      # Re-exports L1 interfaces, main setup logic
│   │   │   └── supervisor.rs # Main SupervisorActor instantiation/management?
│   │   └── Cargo.toml      # Dependencies: janus-interfaces, janus-core, janus-browser-chrome, janus-browser-firefox, etc.
│   │
│   └── janus-plugin-api/   # (Future) Plugin System Interface
│       ├── src/
│       └── Cargo.toml
│
├── examples/             # Example usage accessing janus-client
├── tests/                # Integration & End-to-End tests
└── README.md             # Project overview
```

**Rationale for Crate Structure:**

*   **`janus-interfaces` (L1):** Defines the stable public API (`Browser`, `Page` traits, `ApiError`). Users primarily interact with this. Depends on very little.
*   **`janus-core`:** Contains shared code like configuration handling (`Config`), internal error types, and potentially base actor utilities or the core `SupervisorActor` logic. Avoids duplication across other crates.
*   **`janus-transport` (L3):** Handles the raw network communication (`Transport` trait, specific implementations like WebSocket) and the `ConnectionActor` which manages a single connection. Decouples low-level I/O.
*   **`janus-protocol-handler`:** Centralizes the logic for managing the request/response flow (`CommandActor`) and event distribution (`EventActor`). Browser-specific implementations interact with this crate to send commands and receive events, without needing to manage request IDs or event subscriptions directly.
*   **`janus-browser-*` (L2):** Each crate implements the `janus-interfaces` traits for a specific browser/protocol. Contains the browser/page structs exposed (indirectly) to the user, the corresponding `BrowserActor`/`PageActor` managing state, and the protocol-specific command/event structures and translation logic. Depends on `interfaces`, `core`, and `protocol-handler`.
*   **`janus-client`:** The primary library crate users depend on. It orchestrates the setup (launching browsers, starting actor systems/supervisors), wires the different components together, and re-exports the L1 interfaces for ease of use.
*   **`janus-plugin-api`:** (Future) Defines the interface for external plugins.

This structure allows for clear separation, independent testing of components (e.g., transport layer), and potentially enables users to depend only on the browser implementations they need (if desired, though the main `janus-client` crate likely enables them via features).

## 4. Core Components

### 4.1 Layer Components

#### 4.1.1 L1 - Unified Interface Layer

Provides a stable, protocol-agnostic API.

```rust
// crates/janus-interfaces/src/lib.rs (or browser.rs, page.rs, error.rs)
use serde_json::Value; // Example type

// Represents common errors surfaced to the user
#[derive(Debug /* ... other derives like Error */)]
pub enum ApiError {
    ConnectionFailed(String),
    Timeout,
    ProtocolError(String),
    BrowserCrashed,
    InvalidParameters(String),
    // ... other high-level errors
}

// Represents a unique ID for things like event subscriptions or maybe elements
pub type SubscriptionId = u64; // Example
// Define other common types like ElementHandle, ConsoleMessage etc. here or in common.rs
pub struct ElementHandle { /* ... */ }
pub struct ConsoleMessage { /* ... */ }
pub enum ScreenshotFormat { Png, Jpeg }
pub struct ScreenshotOptions { /* ... */ }


#[async_trait::async_trait] // Add dependency if needed
pub trait Browser: Send + Sync { // Add Send + Sync bounds for multithreading
    // Lifecycle & Connection
    // Note: Connect might be part of a factory/launch function instead of the trait
    // async fn connect(&mut self) -> Result<(), ApiError>;
    async fn disconnect(&mut self) -> Result<(), ApiError>;
    async fn close(&mut self) -> Result<(), ApiError>; // Close the browser process

    // Page Management
    async fn new_page(&self) -> Result<Box<dyn Page>, ApiError>;
    async fn pages(&self) -> Result<Vec<Box<dyn Page>>, ApiError>; // Get handles to existing pages

    // Browser-level operations
    async fn version(&self) -> Result<String, ApiError>;

    // Event Subscription (Example - Requires careful design for async handlers/lifetimes)
    // async fn on_target_created(&self, handler: Box<dyn Fn(Box<dyn Page>) + Send + Sync + 'static>) -> Result<SubscriptionId, ApiError>;
    // async fn unsubscribe(&self, id: SubscriptionId) -> Result<(), ApiError>;
}

#[async_trait::async_trait]
pub trait Page: Send + Sync { // Add Send + Sync bounds
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

    // Input (Examples)
    // async fn click(&self, selector: &str) -> Result<(), ApiError>;
    // async fn type_text(&self, selector: &str, text: &str) -> Result<(), ApiError>;

    // Information
    async fn url(&self) -> Result<String, ApiError>;
    async fn title(&self) -> Result<String, ApiError>;

    // Screenshot
    async fn take_screenshot(&self, format: ScreenshotFormat, options: ScreenshotOptions) -> Result<Vec<u8>, ApiError>;

    // Event Subscription (Page-level - Example)
    // async fn on_load(&self, handler: Box<dyn Fn() + Send + Sync + 'static>) -> Result<SubscriptionId, ApiError>;
    // async fn on_console_message(&self, handler: Box<dyn Fn(ConsoleMessage) + Send + Sync + 'static>) -> Result<SubscriptionId, ApiError>;
}
```

#### 4.1.2 L2 - Browser Implementation Layer

Implements L1 traits using specific browser protocols and actors.

```rust
// crates/janus-browser-chrome/src/browser.rs (Example)
use janus_interfaces::{Browser, Page, ApiError};
use janus_interfaces::common::{/* ... */}; // Import common types if needed
use janus_core::error::InternalError; // Use internal error types
use janus_protocol_handler::command_actor::CommandActor; // Example usage
use janus_protocol_handler::event_actor::EventActor; // Example usage
use janus_transport::connection::ConnectionActor; // Example usage
use janus_transport::ConnectParams; // Parameters for connection
use actix::prelude::{Addr, System}; // Assuming Addr, System from actor framework

use super::page::ChromePage;
use super::actors::{ChromeBrowserActor, SupervisorMsg, BrowserCtrlMsg, PageCtrlMsg};


// Represents the user-facing handle to a Chrome browser instance
pub struct ChromeBrowser {
    // Internal handle to the actor managing this browser instance.
    // NOT exposed directly to the user.
    actor_addr: Addr<ChromeBrowserActor>,
    // Maybe actor system handle if needed for spawning pages? Or managed by Supervisor.
    // system: System,
}

impl ChromeBrowser {
    // This likely lives in janus-client/src/launch.rs or similar
    pub async fn launch(params: ConnectParams /* , config: &Config */) -> Result<Self, ApiError> {
        // 1. Start the Actor System (if not already running globally/per-instance)
        // 2. Start the Supervisor Actor (maybe implicitly started by system?)
        // 3. Supervisor launches ConnectionActor, CommandActor, EventActor
        // 4. Supervisor launches ChromeBrowserActor, providing Addrs of core actors
        // 5. Wait for BrowserActor to signal readiness (connected, initial state fetched)
        // Example (pseudo-code):
        // let system = System::new(); // Or get handle to existing
        // let core_actors = setup_core_actors(&system, &params).await?; // Supervisor internally?
        // let actor_addr = core_actors.supervisor.send(SupervisorMsg::LaunchBrowser(BrowserType::Chrome, params, core_actors.command, core_actors.event)).await??;
        // Ok(Self { actor_addr })
        todo!("Implement browser launching and actor setup in janus-client crate")
    }
}

#[async_trait::async_trait]
impl Browser for ChromeBrowser {
     async fn disconnect(&mut self) -> Result<(), ApiError> {
        // Send Disconnect message to actor_addr
        // Map internal actor/protocol errors to ApiError
        todo!()
     }
     async fn close(&mut self) -> Result<(), ApiError> {
        // Send Close message to actor_addr
        todo!()
     }

    async fn new_page(&self) -> Result<Box<dyn Page>, ApiError> {
        // 1. Send message to BrowserActor (e.g., BrowserCtrlMsg::CreatePage)
        // 2. BrowserActor sends Target.createTarget command via CommandActor
        // 3. BrowserActor receives new target ID/PageActor Addr in response
        // 4. Wrap the Addr in a ChromePage struct
        // 5. Return Box::new(ChromePage { ... })
        let result = self.actor_addr.send(BrowserCtrlMsg::CreatePage).await
            .map_err(|mb_err| ApiError::ProtocolError(format!("Mailbox error: {}", mb_err)))? // Mailbox error
            .map_err(|internal_err| map_internal_to_api_error(internal_err))?; // Actor/Protocol error

        Ok(Box::new(ChromePage::new(result.page_actor_addr, result.page_id)))
    }

    async fn pages(&self) -> Result<Vec<Box<dyn Page>>, ApiError> {
        // 1. Send GetPages message to BrowserActor
        // 2. Actor returns Vec<(Addr<ChromePageActor>, String)>
        // 3. Map this Vec into Vec<Box<dyn Page>>
        todo!()
    }

    async fn version(&self) -> Result<String, ApiError> {
        let result = self.actor_addr.send(BrowserCtrlMsg::GetVersion).await
            .map_err(|mb_err| ApiError::ProtocolError(format!("Mailbox error: {}", mb_err)))?
            .map_err(|internal_err| map_internal_to_api_error(internal_err))?;
        Ok(result)
    }

    // ... other Browser trait methods implemented similarly by sending messages to self.actor_addr ...
}

// Helper function to map internal errors (Actor/Protocol/Transport) to public ApiError
fn map_internal_to_api_error(internal_error: InternalError) -> ApiError {
    match internal_error {
        InternalError::Transport(transport_err) => ApiError::ConnectionFailed(transport_err.to_string()),
        InternalError::Protocol(protocol_err) => ApiError::ProtocolError(protocol_err.message),
        InternalError::Actor(actor_err) => ApiError::ProtocolError(format!("Internal actor error: {}", actor_err)),
        InternalError::Timeout => ApiError::Timeout,
        InternalError::BrowserProcessDied => ApiError::BrowserCrashed,
        InternalError::InvalidParams(msg) => ApiError::InvalidParameters(msg),
        // ... other mappings ...
    }
}

```
```rust
// crates/janus-browser-chrome/src/page.rs (Example)
use janus_interfaces::{Page, ApiError, ElementHandle, ScreenshotFormat, ScreenshotOptions};
use janus_core::error::InternalError;
use super::actors::{ChromePageActor, PageCtrlMsg}; // Assuming PageCtrlMsg contains commands
use super::browser::map_internal_to_api_error; // Reuse error mapping
use actix::prelude::Addr;
use serde_json::Value;

// Represents a handle to a specific Chrome page/target
pub struct ChromePage {
    actor_addr: Addr<ChromePageActor>,
    page_id: String, // Store the ID for the id() method
}

impl ChromePage {
    pub(crate) fn new(actor_addr: Addr<ChromePageActor>, page_id: String) -> Self {
        Self { actor_addr, page_id }
    }
}

#[async_trait::async_trait]
impl Page for ChromePage {
     async fn navigate(&self, url: &str) -> Result<(), ApiError> {
         let msg = PageCtrlMsg::Navigate { url: url.to_string() };
         self.actor_addr.send(msg).await
            .map_err(|mb_err| ApiError::ProtocolError(format!("Mailbox error: {}", mb_err)))? // Handle mailbox error first
            .map_err(map_internal_to_api_error) // Then handle logical error from actor
     }

     async fn close(&self) -> Result<(), ApiError> {
         self.actor_addr.send(PageCtrlMsg::Close).await
            .map_err(|mb_err| ApiError::ProtocolError(format!("Mailbox error: {}", mb_err)))?
            .map_err(map_internal_to_api_error)
     }

     fn id(&self) -> String {
         self.page_id.clone()
     }

     async fn evaluate_script(&self, script: &str) -> Result<Value, ApiError> {
         let msg = PageCtrlMsg::EvaluateScript { script: script.to_string() };
         self.actor_addr.send(msg).await
            .map_err(|mb_err| ApiError::ProtocolError(format!("Mailbox error: {}", mb_err)))?
            .map_err(map_internal_to_api_error)
     }

     // ... other Page trait methods implemented by sending specific PageCtrlMsg variants ...
     async fn content(&self) -> Result<String, ApiError> { todo!() }
     async fn reload(&self) -> Result<(), ApiError> { todo!() }
     async fn go_back(&self) -> Result<(), ApiError> { todo!() }
     async fn go_forward(&self) -> Result<(), ApiError> { todo!() }
     async fn call_function(&self, function_declaration: &str, args: Vec<Value>) -> Result<Value, ApiError> { todo!() }
     async fn query_selector(&self, selector: &str) -> Result<Option<ElementHandle>, ApiError> { todo!() }
     async fn wait_for_selector(&self, selector: &str, timeout_ms: u64) -> Result<ElementHandle, ApiError> { todo!() }
     async fn url(&self) -> Result<String, ApiError> { todo!() }
     async fn title(&self) -> Result<String, ApiError> { todo!() }
     async fn take_screenshot(&self, format: ScreenshotFormat, options: ScreenshotOptions) -> Result<Vec<u8>, ApiError> { todo!() }

}
```

#### 4.1.3 L3 - Transport & Connection Layer

Manages the raw connection and message transport.

```rust
// crates/janus-transport/src/traits.rs
use async_trait::async_trait;
use crate::error::TransportError; // Use TransportError defined in janus-core or janus-transport

#[async_trait]
pub trait Transport: Send + Unpin { // Require Send + Unpin for async tasks
    async fn connect(&mut self) -> Result<(), TransportError>;
    async fn disconnect(&mut self) -> Result<(), TransportError>;
    // Send borrowed data to potentially avoid clones
    async fn send(&mut self, message: &str) -> Result<(), TransportError>;
    // Receive owned String or error
    async fn receive(&mut self) -> Option<Result<String, TransportError>>;
}

// crates/janus-transport/src/connection.rs
use actix::prelude::*;
use tokio::sync::mpsc; // Example channel type
use crate::error::TransportError;
use crate::traits::Transport;
use crate::ConnectParams; // Assume defined here or in lib.rs
use crate::websocket::WebSocketTransport; // Example concrete transport


// Actor responsible for managing a single connection
pub struct ConnectionActor {
    // Transport is now created and managed within the connection task
    // transport: Option<Box<dyn Transport>>, // No longer stored directly in actor state
    params: ConnectParams,
    state: ConnectionState,
    // Recipient for successfully received messages (likely CommandActor or EventActor via a dispatcher)
    message_handler: Recipient<IncomingMessage>,
    // Potentially Recipient for sending outgoing messages, managed by CommandActor?
    // Or handle SendMessage directly here.
    outgoing_tx: Option<mpsc::Sender<String>>, // Channel to send messages to the write task
    // Supervisor or parent actor for reporting critical errors/state changes
    supervisor: Recipient<ConnectionStatusUpdate>, // Report status updates
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
        }
    }
}

// Message to send out via the connection
#[derive(Message, Clone)] // Clone might be useful
#[rtype(result = "Result<(), TransportError>")]
pub struct SendMessage(pub String);

// Message received from transport, forwarded to message_handler
#[derive(Message)]
#[rtype(result = "()")]
pub struct IncomingMessage(pub String);

// Internal message to update actor state based on transport events
#[derive(Message)]
#[rtype(result = "()")]
struct TransportEvent(ConnectionState); // Renamed from ConnectionStatusUpdate

// Message sent TO supervisor/parent actor
#[derive(Message, Debug, Clone)]
#[rtype(result = "()")]
pub struct ConnectionStatusUpdate(pub ConnectionState);


#[derive(Debug, Clone, PartialEq)] // Add derives for state management
pub enum ConnectionState {
    Idle,
    Connecting,
    Connected,
    Disconnecting,
    Disconnected(Option<TransportError>)
}

impl Actor for ConnectionActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        log::info!("ConnectionActor starting for {}", self.params.url);
        if self.state != ConnectionState::Idle {
            log::warn!("ConnectionActor started in unexpected state: {:?}", self.state);
            return; // Avoid restarting connection logic if already active
        }
        self.state = ConnectionState::Connecting;
        self.supervisor.do_send(ConnectionStatusUpdate(self.state.clone())).ok(); // Notify supervisor

        // Use factory function from transport crate
        let transport_builder = create_transport(&self.params);
        let addr = ctx.address();
        let message_handler = self.message_handler.clone();
        let supervisor_recipient = self.supervisor.clone();

        // Channel for sending messages to the transport write task
        let (outgoing_tx, mut outgoing_rx) = mpsc::channel::<String>(100); // Buffer size configurable
        self.outgoing_tx = Some(outgoing_tx);


        ctx.spawn(async move {
            match transport_builder.connect().await {
                Ok(mut transport) => {
                    log::info!("Transport connected successfully.");
                    addr.do_send(TransportEvent(ConnectionState::Connected));

                    // Split the transport potentially if underlying lib supports it (e.g. WebSocket split)
                    // Otherwise, manage reads and writes in separate tasks or select! loop

                    // Read loop
                    let read_task = async {
                        while let Some(msg_result) = transport.receive().await {
                            match msg_result {
                                Ok(msg) => {
                                    // Forward successfully received messages
                                    if message_handler.do_send(IncomingMessage(msg)).is_err() {
                                         log::error!("Failed to forward incoming message to handler. Actor likely stopped.");
                                         break; // Stop reading if handler is dead
                                    }
                                }
                                Err(e) => {
                                    log::error!("Transport receive error: {}", e);
                                    addr.do_send(TransportEvent(ConnectionState::Disconnected(Some(e))));
                                    break; // Exit read loop on error
                                }
                            }
                        }
                         // If loop exits without error, connection closed gracefully (usually None from receive)
                        log::info!("Transport read loop finished.");
                        // Ensure disconnected state is signalled if not already done by an error
                        addr.do_send(TransportEvent(ConnectionState::Disconnected(None)));
                    };

                    // Write loop
                    let write_task = async {
                        while let Some(msg_to_send) = outgoing_rx.recv().await {
                             if let Err(e) = transport.send(&msg_to_send).await {
                                log::error!("Transport send error: {}", e);
                                // Don't necessarily break the write loop, but report error maybe?
                                // If send fails critically, the read side will likely detect disconnection.
                                // Or signal disconnection directly:
                                addr.do_send(TransportEvent(ConnectionState::Disconnected(Some(e))));
                                break; // Exit write loop on critical send error
                             }
                        }
                        log::info!("Transport write loop finished.");
                        // Ensure transport disconnect is called when write loop ends
                        if let Err(e) = transport.disconnect().await {
                            log::warn!("Error during transport disconnect: {}", e);
                        }
                    };

                    // Run both tasks concurrently
                    tokio::select! {
                        _ = read_task => { log::info!("Read task completed."); },
                        _ = write_task => { log::info!("Write task completed."); },
                    }

                },
                Err(e) => {
                    log::error!("Transport connect error: {}", e);
                    // Send Disconnected state back to self to handle state update and notify supervisor
                    addr.do_send(TransportEvent(ConnectionState::Disconnected(Some(e))));
                }
            }
            log::info!("Connection task finished for {}", addr.connected());
        }.into_actor(self));
    }

    fn stopping(&mut self, _ctx: &mut Self::Context) -> Running {
        log::info!("ConnectionActor stopping.");
        // Close the outgoing channel to signal the write task to stop
        self.outgoing_tx.take();
        // Set state to Disconnecting or Disconnected
        if self.state != ConnectionState::Disconnected(None) && !matches!(self.state, ConnectionState::Disconnected(Some(_))) {
             self.state = ConnectionState::Disconnecting;
             self.supervisor.do_send(ConnectionStatusUpdate(self.state.clone())).ok();
        }
        Running::Stop
    }
}


// Handler for internal state updates from the connection task
impl Handler<TransportEvent> for ConnectionActor {
    type Result = ();
    fn handle(&mut self, msg: TransportEvent, ctx: &mut Context<Self>) {
        let new_state = msg.0;
        if self.state == new_state { return; } // Avoid redundant updates

        log::info!("Connection state changing from {:?} to {:?}", self.state, new_state);
        self.state = new_state.clone();

        // Notify supervisor about the state change
        if self.supervisor.do_send(ConnectionStatusUpdate(new_state.clone())).is_err() {
             log::warn!("Failed to send status update to supervisor. It might have stopped.");
        }

        // If disconnected, stop the actor or handle reconnection logic
        if let ConnectionState::Disconnected(ref _err) = new_state {
            log::warn!("ConnectionActor moving to Disconnected state.");
            // Clear the sender channel
             self.outgoing_tx.take();
             // Optionally: Initiate reconnection logic here or signal supervisor to do so.
             // For now, just stop the actor.
             ctx.stop();
        }
         if new_state == ConnectionState::Connected {
             // Connection is up, potentially notify others or perform actions
         }
    }
}


impl Handler<SendMessage> for ConnectionActor {
    type Result = ResponseActFuture<Self, Result<(), TransportError>>;

    fn handle(&mut self, msg: SendMessage, _ctx: &mut Context<Self>) -> Self::Result {
        let fut = async move {
            match self.state {
                ConnectionState::Connected => {
                    if let Some(tx) = &self.outgoing_tx {
                        if tx.send(msg.0).await.is_ok() {
                            Ok(())
                        } else {
                            log::error!("Outgoing message channel closed unexpectedly.");
                            Err(TransportError::NotConnected("Message channel closed".into())) // Or a specific channel error
                        }
                    } else {
                         log::error!("Attempted to send message but outgoing channel is missing.");
                         Err(TransportError::NotConnected("Internal channel missing".into()))
                    }
                },
                _ => {
                    log::warn!("Attempted to send message while not connected (State: {:?})", self.state);
                    Err(TransportError::NotConnected(format!("Current state: {:?}", self.state)))
                }
            }
        };
        // Wrap the future in a ResponseActFuture
        Box::pin(fut.into_actor(self))
    }
}

// Factory function should live in transport crate's lib.rs or mod.rs
// Helper function (outside actor) - Now part of janus-transport crate
fn create_transport(params: &ConnectParams) -> Box<dyn Transport> {
    // Logic to choose WebSocket, TCP etc. based on params.url scheme
    #[cfg(feature = "websocket")]
    if params.url.starts_with("ws://") || params.url.starts_with("wss://") {
         log::debug!("Creating WebSocketTransport for URL: {}", params.url);
         // Pass relevant parts of params or the whole thing
         return Box::new(WebSocketTransport::new(params.clone())); // Assuming WebSocketTransport::new takes ConnectParams
    }
    // ... other transport types based on features or URL scheme ...

    // Fallback or panic if no suitable transport found
    panic!("Unsupported transport scheme or feature not enabled for URL: {}", params.url);
}

// Define ConnectParams in the transport crate as well
// crates/janus-transport/src/lib.rs (or types.rs)
#[derive(Clone, Debug)] // Needed for cloning into async task
pub struct ConnectParams {
    pub url: String,
    #[cfg(feature = "websocket")]
    pub ws_options: Option<WebSocketConnectOptions>, // Example: Specific options
    pub timeout: std::time::Duration,
}

#[derive(Clone, Debug, Default)]
#[cfg(feature = "websocket")]
pub struct WebSocketConnectOptions {
    pub max_message_size: Option<usize>,
    pub accept_unmasked_frames: bool,
    // ... other tokio-tungstenite or specific options ...
}

// Define TransportError in janus-core or janus-transport crate
// crates/janus-core/src/error.rs (or crates/janus-transport/src/error.rs)
use thiserror::Error;

#[derive(Error, Debug, Clone)] // Clone might be useful if error needs to be stored/passed
pub enum TransportError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Not connected: {0}")]
    NotConnected(String),
    #[error("Send operation failed: {0}")]
    SendFailed(String),
    #[error("Receive operation failed: {0}")]
    ReceiveFailed(String),
    #[error("Serialization/Deserialization error: {0}")]
    SerdeError(String),
    #[error("Connection timed out")]
    Timeout,
    #[error("Invalid URL scheme: {0}")]
    InvalidScheme(String),
    #[error("IO error: {0}")]
    Io(String), // Wrap std::io::Error or other IO errors
    #[error("WebSocket protocol error: {0}")]
    WebSocketError(String),
     #[error("TLS error: {0}")]
    TlsError(String),
    #[error("Operation cancelled")]
    Cancelled,
     #[error("Unknown transport error: {0}")]
    Other(String),
}

// Implement From<std::io::Error> etc. as needed for convenience

```

### 4.2 Actor System Design

#### 4.2.1 Overview and Rationale

The actor model provides:
*   **Concurrency:** Handles multiple simultaneous operations (e.g., commands, event processing) without manual lock management.
*   **State Encapsulation:** Each actor manages its own state, preventing race conditions.
*   **Asynchronous Communication:** Messages are sent asynchronously, fitting well with the `async/await` paradigm.
*   **Fault Isolation:** Errors in one actor can be contained and managed by its supervisor without necessarily crashing the entire application.

#### 4.2.2 Key Actors and Responsibilities

*   **`Supervisor Actor` (Core - likely in `janus-client` or `janus-core`):**
    *   Responsibilities: Top-level management, launching and overseeing core actors (`CommandActor`, `EventActor`, `ConnectionActor`) and browser-specific actors (`BrowserActor`), defining supervision strategies (restart, stop, escalate on child failure), potentially managing global resources or configuration, handling `ConnectionStatusUpdate` messages.
    *   State: Addresses of child actors it supervises, overall system state.
*   **`Browser Actor` (L2 - Per Browser Instance - in `janus-browser-*` crates):**
    *   Responsibilities: Manages the lifecycle of a specific browser *instance* (if applicable, e.g., launching the process), handles browser-level protocol commands (e.g., `Browser.getVersion`, `Target.createTarget`) by sending messages to `CommandActor`, manages associated `PageActor`s, maintains overall browser state (`Ready`, `Crashed`), receives relevant browser-level events via `EventActor`.
    *   State: `BrowserState`, Map of `TargetId -> Addr<PageActor>`, `Addr<CommandActor>`, `Addr<EventActor>`, potentially browser process info.
*   **`Page Actor` (L2 - Per Browser Tab/Target - in `janus-browser-*` crates):**
    *   Responsibilities: Manages the state and operations for a single browser page or target. Handles page-specific protocol commands (e.g., `Page.navigate`, `Runtime.evaluate`, `DOM.querySelector`) by sending messages to `CommandActor`. Subscribes to relevant page-level events (e.g., `Page.loadEventFired`, `Runtime.consoleAPICalled`) via `EventActor`. Implements the logic backing the L1 `Page` trait methods, returning results/errors.
    *   State: `PageState`, `TargetId`/`SessionId`, URL, `Addr<CommandActor>`, `Addr<EventActor>`.
*   **`Connection Actor` (L3 - Per Connection - in `janus-transport`):**
    *   Responsibilities: Manages a *single* underlying transport connection (WebSocket, TCP, etc.). Handles connecting, disconnecting, sending raw protocol messages (via `SendMessage`), receiving raw messages, reporting transport-level errors and state changes via `ConnectionStatusUpdate` to its supervisor. Forwards successfully received messages via `IncomingMessage` to a designated handler (likely `CommandActor` or `EventActor`).
    *   State: `ConnectionState`, `ConnectParams`, `Addr` of message handler, `Addr` of supervisor.
*   **`Command Actor` (Core/Protocol - in `janus-protocol-handler`):**
    *   Responsibilities: **Orchestrates command execution.** Receives structured command requests (e.g., `SendCommand { target_id, method, params, reply_to }` from `BrowserActor`/`PageActor`), translates them into the specific wire protocol format (e.g., JSON-RPC for CDP), assigns unique message IDs, sends the formatted message via the `ConnectionActor`'s `SendMessage`, tracks pending requests (maps message ID to `reply_to` actor/future), receives incoming messages (`IncomingMessage` via dispatcher) potentially containing responses, matches responses to pending requests by ID, handles command timeouts, parses the response/error from the browser, and sends the result (or `InternalError`) back to the original requester (`reply_to`). May also act as the primary `message_handler` for `ConnectionActor`, dispatching events to `EventActor`.
    *   State: Map of `RequestId -> PendingRequestInfo (reply_to, timeout_timer)`, `RequestId` counter, `Addr<ConnectionActor>`, `Addr<EventActor>`.
*   **`Event Actor` (Core/Protocol - in `janus-protocol-handler`):**
    *   Responsibilities: Receives parsed protocol events (forwarded by `CommandActor` or directly from `ConnectionActor`), parses/categorizes events based on their type/domain/target (e.g., `Page.loadEventFired` for target X), manages event subscriptions from various actors (`BrowserActor`, `PageActor`, `PluginActor`, L1 client handlers via adapters), and distributes events only to registered subscribers interested in that specific event type/target.
    *   State: Map of `(EventType, Option<TargetId>) -> Vec<SubscriberAddress>`, potentially an event buffer.
*   **`Plugin Actor` (L2 - Per Plugin Instance - Potentially in separate `janus-plugin-*` crates):**
    *   Responsibilities: Manages the lifecycle and execution of a single plugin. Interacts with other actors (`CommandActor`, `EventActor`) to observe or control the browser based on plugin logic. Subscribes to events via `EventActor`. Sends commands via `CommandActor`.
    *   State: Plugin-specific state, `PluginId`, `Addr<CommandActor>`, `Addr<EventActor>`.
*   **`Monitor Actor` (Core - Optional Singleton - in `janus-core` or `janus-client`):**
    *   Responsibilities: Gathers metrics (actor mailbox sizes, message throughput, command latency), monitors actor health (e.g., via heartbeats or supervision), logs system-level diagnostic information. Could subscribe to specific events or status updates.
    *   State: Metrics data, health status of monitored actors.

#### 4.2.3 Message Flow

Visualizing the interactions between actors and components using sequence diagrams helps clarify the flow.

*   **Command Execution Flow:**

    ```mermaid
    sequenceDiagram
        participant Client
        participant L2Impl as L2 Impl (e.g., ChromePage)
        participant PageActor as Page Actor (e.g., ChromePageActor)
        participant CommandActor as Command Actor
        participant ConnActor as Connection Actor
        participant Transport
        participant Browser

        Client->>+L2Impl: Call L1 method (e.g., page.navigate("url"))
        L2Impl->>+PageActor: Send internal Ctrl Msg (e.g., PageCtrlMsg::Navigate)
        PageActor->>+CommandActor: Send SendCommand { method, params, reply_to: self }
        CommandActor->>CommandActor: Generate ID, Serialize Cmd, Store Pending Request
        CommandActor->>+ConnActor: Send SendMessage(json_string)
        ConnActor->>+ConnActor: Forward to Write Task (via channel)
        Note over ConnActor: Write Task sends over socket
        ConnActor->>+Transport: Send raw message bytes
        Transport->>+Browser: Deliver command
        Browser-->>-Transport: (Response comes later)
        Transport-->>-ConnActor:
        ConnActor-->>-CommandActor:
        CommandActor-->>-PageActor:
        PageActor-->>-L2Impl:
        L2Impl-->>-Client:
    ```

*   **Response Handling Flow:**

    ```mermaid
    sequenceDiagram
        participant Browser
        participant Transport
        participant ConnActor as Connection Actor
        participant CommandActor as Command Actor
        participant PageActor as Page Actor (Requester)
        participant L2Impl as L2 Impl
        participant Client

        Browser->>+Transport: Send response message (with ID)
        Transport->>+ConnActor: Receive raw message bytes
        Note over ConnActor: Read Task receives and sends internally
        ConnActor->>+CommandActor: Send IncomingMessage(response_string)
        CommandActor->>CommandActor: Parse JSON, Find Pending Request by ID, Cancel Timeout
        CommandActor->>+PageActor: Send Result<Value, InternalError> (via stored reply_to Addr)
        PageActor->>PageActor: Process result
        PageActor-->>-L2Impl: Return Result (resolves await)
        L2Impl->>+L2Impl: Map InternalError to ApiError if needed
        L2Impl-->>-Client: Return final Result<_, ApiError>
    ```

*   **Event Handling Flow:**

    ```mermaid
    sequenceDiagram
        participant Browser
        participant Transport
        participant ConnActor as Connection Actor
        participant CommandActor as Command Actor (Dispatcher)
        participant EventActor as Event Actor
        participant SubscribedActor as Subscribed Actors (e.g., PageActor, PluginActor)

        Browser->>+Transport: Send event message (no ID)
        Transport->>+ConnActor: Receive raw message bytes
        Note over ConnActor: Read Task receives and sends internally
        ConnActor->>+CommandActor: Send IncomingMessage(event_string)
        CommandActor->>CommandActor: Parse JSON, Identify as event (no ID)
        CommandActor->>+EventActor: Forward ProtocolEvent(json_value)
        EventActor->>EventActor: Parse event details (method, params, targetId?), Find subscribers
        EventActor->>+SubscribedActor: Dispatch Parsed Event (e.g., PageLoadEvent)
        SubscribedActor->>SubscribedActor: Handle event (e.g., update state)
        SubscribedActor-->>-EventActor: (Ack Optional)
        EventActor-->>-CommandActor: (Ack Optional)
        CommandActor-->>-ConnActor: (Ack Optional)
        ConnActor-->>-Transport:
        Transport-->>-Browser:
    ```

#### 4.2.4 Actor State Management

*   **Generic Actor State:** Actors may internally track lifecycle state: `Starting`, `Running`, `Stopping`, `Failed(Error)`. Supervisors use this.
*   **Domain-Specific State:** Key actors manage state relevant to their domain:
    *   `ConnectionActor`: `ConnectionState` (Idle, Connecting, Connected, Disconnecting, Disconnected). Managed via `TransportEvent` messages.
    *   `BrowserActor`: `BrowserState` (Initializing, Ready, Degraded, Crashed, Closed). Updated based on its own logic and events received from `EventActor`.
    *   `PageActor`: `PageState` (Loading, Interactive, Complete, Crashed, Closed), Current URL, Target ID. Updated based on command results and events received from `EventActor`.
*   State transitions often trigger notifications or internal logic changes (e.g., `ConnectionState::Connected` enables message sending via `CommandActor`, `ConnectionState::Disconnected` triggers supervisor notification and actor shutdown).

#### 4.2.5 Actor Error Handling & Supervision

*   **Actor Internal Errors:** Handled within the actor's message handlers. Request-response messages return `Result<OkType, InternalError>`. Mailbox errors are handled by the caller (`.await.map_err(...)`).
*   **Transport Errors:** Primarily detected by `ConnectionActor`'s read/write/connect tasks. Reported internally via `TransportEvent(ConnectionState::Disconnected(Some(err)))`. The `ConnectionActor` then updates its state and notifies its `Supervisor` via `ConnectionStatusUpdate`.
*   **Protocol Errors:**
    *   *Browser-reported errors:* Received as part of a response message (e.g., JSON-RPC error object). Parsed by `CommandActor` and returned as `InternalError::Protocol` to the requesting actor (`PageActor`/`BrowserActor`).
    *   *Serialization/Deserialization errors:* Can occur in `CommandActor` (sending/receiving) or `EventActor` (receiving). Logged and potentially returned as `InternalError::Protocol` or a specific Serde error variant.
    *   *Malformed Events/Responses:* Handled by `CommandActor` / `EventActor` during parsing. Logged; potentially ignored if non-critical or reported if indicating a protocol mismatch.
*   **Actor Panics / Critical Errors:** Caught by the actor framework (`actix`, `ractor`, etc.) and reported to the `Supervisor Actor`.
*   **Supervision Strategy:** The `Supervisor Actor` (likely in `janus-client`) defines how to react to child actor failures (based on the actor and error type):
    *   `Restart`: Attempt to restart the failed actor (and potentially its dependencies). Useful for recoverable errors like a `ConnectionActor` failure if the browser is still running.
    *   `Stop`: Stop the failed actor and potentially related actors (e.g., if `BrowserActor` crashes, stop its `PageActor`s and maybe `ConnectionActor`).
    *   `Escalate`: Propagate the error up the supervision hierarchy (if nested supervisors exist). Usually leads to stopping a larger part of the system.
*   **Propagation to L1:** Critical, unrecoverable errors managed by the supervisor (e.g., browser process crash detected by `BrowserActor` monitoring, persistent `ConnectionActor` failure, critical failure in `CommandActor`) should eventually cause:
    1.  The relevant `BrowserActor`/`PageActor` to enter a failed state.
    2.  Subsequent calls to the corresponding L1 `Browser`/`Page` methods (handled by `ChromeBrowser`/`ChromePage`) to fail quickly, returning an appropriate `ApiError` (e.g., `ApiError::BrowserCrashed`, `ApiError::ConnectionFailed`) derived from the internal state or error received from the actor.
    3.  Ongoing operations (like pending command futures) associated with the failed component should be cancelled and return an appropriate `ApiError`.

### 4.3 Configuration System

Configuration can be loaded from files (e.g., TOML, YAML) or environment variables, likely managed within the `janus-core` crate.

```rust
// crates/janus-core/src/config.rs
use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf, time::Duration};

// Main configuration structure
#[derive(Deserialize, Debug, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub global: GlobalConfig,
    #[serde(default)]
    pub transport: TransportConfig,
    #[serde(default)]
    pub actor_system: ActorSystemConfig,
    #[serde(default)]
    pub browser_defaults: BrowserLaunchConfig, // Default launch settings
    #[serde(default)]
    pub browsers: HashMap<String, BrowserLaunchConfig>, // Browser-specific overrides (e.g., "chrome", "firefox")
}

// Global settings
#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct GlobalConfig {
    pub log_level: String, // e.g., "info", "debug", "trace"
    pub default_command_timeout: Duration, // Default timeout for protocol commands
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            default_command_timeout: Duration::from_secs(30),
        }
    }
}

// Transport layer configuration
#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct TransportConfig {
    pub connect_timeout: Duration,
    #[cfg(feature = "websocket")]
    pub websocket: WebSocketConfig,
    // pub tcp: TcpConfig, // If TCP transport is added
    // pub ipc: IpcConfig, // If IPC transport is added
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(20),
            #[cfg(feature = "websocket")]
            websocket: Default::default(),
        }
    }
}

// WebSocket specific configuration
#[derive(Deserialize, Debug, Clone, Default)]
#[cfg(feature = "websocket")]
#[serde(default)]
pub struct WebSocketConfig {
    pub max_message_size: Option<usize>,
    pub max_frame_size: Option<usize>,
    pub accept_unmasked_frames: bool, // For some older protocols/proxies
    // Other relevant settings from tungstenite/tokio-tungstenite
}

// Actor system tuning parameters
#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct ActorSystemConfig {
    pub default_mailbox_capacity: usize,
    // Could add thread pool sizes (if applicable to chosen framework)
    // Supervision strategy defaults? (complex to represent in config)
}

impl Default for ActorSystemConfig {
     fn default() -> Self {
         Self { default_mailbox_capacity: 100 }
     }
}

// Configuration for launching and connecting to a browser instance
#[derive(Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct BrowserLaunchConfig {
    // Launch options (if managing the browser process)
    pub executable_path: Option<PathBuf>, // Path to browser executable
    pub user_data_dir: Option<PathBuf>,  // Profile directory
    pub headless: Option<bool>,           // Run in headless mode
    pub args: Option<Vec<String>>,       // Extra command-line arguments
    pub env_vars: Option<HashMap<String, String>>, // Environment variables for process

    // Connection options (if connecting to an existing browser or launched process)
    pub remote_debugging_address: Option<String>, // e.g., "127.0.0.1"
    pub remote_debugging_port: Option<u16>,       // e.g., 9222 (CDP), TBD (BiDi)
    pub connection_url_override: Option<String>, // Explicit WS/TCP URL (takes precedence)
    pub protocol: Option<BrowserProtocol>,       // Specify CDP or BiDi (might be auto-detected)

    // Protocol-specific settings
    pub cdp_settings: Option<CdpSettings>,
    pub bidi_settings: Option<BidiSettings>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum BrowserProtocol {
    #[serde(rename = "cdp")]
    CDP,
    #[serde(rename = "bidi")]
    WebDriverBiDi,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct CdpSettings {
    pub use_flattened_target_info: bool, // Use Target.getTargetInfo vs Target.getTargets for session mapping
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct BidiSettings {
    // WebDriver BiDi specific configuration options
    pub capabilities: Option<serde_json::Value>, // WebDriver capabilities JSON
}

// Helper to merge default and specific configs
impl BrowserLaunchConfig {
    pub fn merged_with(&self, defaults: &BrowserLaunchConfig) -> Self {
        Self {
            executable_path: self.executable_path.clone().or_else(|| defaults.executable_path.clone()),
            user_data_dir: self.user_data_dir.clone().or_else(|| defaults.user_data_dir.clone()),
            headless: self.headless.or(defaults.headless),
            args: self.args.clone().or_else(|| defaults.args.clone()),
            env_vars: self.env_vars.clone().or_else(|| defaults.env_vars.clone()),
            remote_debugging_address: self.remote_debugging_address.clone().or_else(|| defaults.remote_debugging_address.clone()),
            remote_debugging_port: self.remote_debugging_port.or(defaults.remote_debugging_port),
            connection_url_override: self.connection_url_override.clone().or_else(|| defaults.connection_url_override.clone()),
            protocol: self.protocol.clone().or_else(|| defaults.protocol.clone()),
            cdp_settings: self.cdp_settings.clone().or_else(|| defaults.cdp_settings.clone()),
            bidi_settings: self.bidi_settings.clone().or_else(|| defaults.bidi_settings.clone()),
        }
    }
}


// Function to load configuration (example)
use config::{Config as ConfigLoader, File, Environment}; // Using the `config` crate

pub fn load_config() -> Result<Config, config::ConfigError> {
    let builder = ConfigLoader::builder()
        // Add default values
        .set_default("global.log_level", "info")?
        .set_default("global.default_command_timeout_ms", 30000)? // Use ms here for TOML friendliness
        .set_default("transport.connect_timeout_ms", 20000)?
        // ... other defaults ...

        // Load from a configuration file `janus.toml` (optional)
        .add_source(File::with_name("janus").required(false))
        // Load from environment variables with prefix `JANUS_` (e.g., JANUS_GLOBAL_LOG_LEVEL)
        .add_source(Environment::with_prefix("JANUS").separator("_"))
        .build()?;

    // Deserialize the loaded configuration
    builder.try_deserialize()
}


// Need to handle Duration deserialization properly if using ms in config file
// (e.g., using serde helper attributes like `#[serde(with = "humantime_serde")]` or custom deserializer)

```

## 5. Implementation Plan

This plan reflects the proposed crate structure.

### Phase 1: Core Foundation (Crates: `janus-interfaces`, `janus-core`, `janus-transport`)
- [ ] Setup workspace structure (`Cargo.toml` for workspace).
- [ ] **`janus-interfaces`:** Define core L1 traits (`Browser`, `Page`) and `ApiError`. Define common data types (`ElementHandle`, etc.).
- [ ] **`janus-core`:** Implement configuration loading (`Config` struct using `config` crate). Define internal error types (`InternalError`, `TransportError`). Setup logging (`log`, `env_logger`/`tracing`). Basic actor traits/messages if needed centrally.
- [ ] **`janus-transport`:** Define `Transport` trait. Implement basic `ConnectionActor` structure (state machine, message types `SendMessage`, `IncomingMessage`, `ConnectionStatusUpdate`). Implement `WebSocketTransport` using `tokio-tungstenite`. Implement `ConnectParams`. Setup transport features (`websocket`). Define `TransportError` (or move from core).
- [ ] **`janus-client`:** Setup main library crate structure. Basic `SupervisorActor` skeleton (responsible for launching core actors).

### Phase 2: Protocol Handling & Chrome CDP MVP (Crates: `janus-protocol-handler`, `janus-browser-chrome`)
- [ ] **`janus-protocol-handler`:** Implement `CommandActor` logic (request ID generation, pending request tracking (HashMap), timeout handling, basic JSON-RPC serialization, routing responses to `reply_to`). Implement `EventActor` logic (subscription map `(EventType, TargetId?) -> Vec<Addr>`, basic event dispatch). Define `SendCommand`, `ProtocolEvent` messages. Implement logic for `CommandActor` to dispatch incoming messages (responses vs events).
- [ ] **`janus-transport`:** Refine `ConnectionActor` to integrate with `CommandActor`/`EventActor` (forwarding `IncomingMessage`, receiving `SendMessage`). Implement the connection task loop (connect, read/write loops, error handling, state reporting to Supervisor).
- [ ] **`janus-browser-chrome`:** Define `ChromeBrowserActor` and `ChromePageActor` skeletons. Define actor messages (`BrowserCtrlMsg`, `PageCtrlMsg`). Implement L2 `ChromeBrowser` struct implementing basic `Browser` methods (`connect` (via launch), `disconnect`, `version`, `new_page`). Implement L2 `ChromePage` struct implementing basic `Page` methods (`navigate`, `evaluate_script`, `close`). These methods will send messages to their respective actors.
- [ ] **`janus-browser-chrome` / `janus-protocol-handler`:** Implement CDP-specific command/event serialization/deserialization logic within `Chrome*Actor`s or helper functions, interacting with `CommandActor` via `SendCommand`. Implement basic event subscription logic (`PageActor` subscribes to page events via `EventActor`).
- [ ] **`janus-client`:** Implement basic browser launching logic (`launch_chrome`) which sets up the `SupervisorActor`, core actors (`Connection`, `Command`, `Event`), and the `ChromeBrowserActor`. Wire them together correctly.

### Phase 3: Features, Robustness & Testing
- [ ] **Error Handling:** Implement robust error handling across all layers. Implement supervisor strategies in `SupervisorActor`. Ensure errors correctly propagate to L1 `ApiError`s. Handle browser crashes / unexpected disconnects.
- [ ] **Features:** Implement the full set of L1 `Browser` and `Page` methods for Chrome (DOM manipulation, screenshots, network events, etc.).
- [ ] **Event Handling:** Implement robust event handling, allowing L1 users to subscribe to events (requires careful API design for async callbacks/streams).
- [ ] **State Management:** Implement proper session/target management within `ChromeBrowserActor` (handling target creation/destruction events). Refine `PageState` / `BrowserState`.
- [ ] **Testing:** Add comprehensive unit tests for actor logic, protocol helpers, config parsing. Add integration tests for actor interactions (mocking `ConnectionActor`'s transport). Add basic E2E tests launching Chrome and performing simple operations.
- [ ] **`janus-core` / `janus-client`:** Develop optional `MonitorActor` for basic metrics (e.g., command latency, actor mailbox size).

### Phase 4: Additional Browsers & Polish
- [ ] **`janus-browser-firefox`:** Add Firefox support (WebDriver BiDi). Create new crate, implement L2 `FirefoxBrowser/Page` and corresponding Actors. Adapt `janus-protocol-handler` if necessary for BiDi's structure (likely needs different serialization/response handling than CDP). Requires new BiDi-specific protocol types.
- [ ] **`janus-browser-edge`:** Add Edge support (likely reusing much of `janus-browser-chrome`'s CDP implementation, potentially via shared internal crate or feature flags).
- [ ] **Plugin System:** Design and implement `janus-plugin-api` crate. Implement `PluginActor` within `janus-protocol-handler` or `janus-client`. Allow plugins to interact via `CommandActor`/`EventActor`.
- [ ] **Testing:** Add E2E tests for Firefox/Edge. Add more complex E2E scenarios.
- [ ] **Performance:** Benchmarking and optimization (message passing, serialization, potential bottlenecks).
- [ ] **Documentation:** Refine all documentation (User, Developer, API). Add examples.

## 6. Testing Strategy

*   **Unit Testing (within each crate):**
    *   Test individual actor message handling logic (`handle` methods) using mock messages, context, and state assertions (`actix::test`, `ractor::test`, or framework equivalents).
    *   Test helper functions (e.g., protocol serialization/deserialization in `janus-browser-*`, config merging in `janus-core`).
    *   Test `janus-transport` implementations with mock streams.
    *   Test `CommandActor` request tracking, ID generation, timeout logic.
    *   Test `EventActor` subscription management and dispatch logic.
*   **Integration Testing (typically within `tests/` directory of relevant crates or workspace `tests/`):**
    *   Test interaction flows between actors within a crate or across crate boundaries (e.g., `PageActor` -> `CommandActor` -> `ConnectionActor`). Start partial actor systems for these tests.
    *   Test `SupervisorActor` strategies by simulating child actor failures.
    *   Test L1 interface calls through the L2 implementation to actors, mocking the transport layer (`ConnectionActor` sending/receiving predefined data instead of connecting). Focus on the L2 -> Actor -> L3(mocked) interaction.
    *   Test configuration loading and application.
*   **End-to-End Testing (workspace `tests/` directory):**
    *   Use test fixtures that launch a real browser instance (e.g., using `WebDriver` manager tools or pre-installed browsers).
    *   Use the main `janus-client` crate's public API (`launch_chrome`, `browser.new_page`, etc.).
    *   Execute common user scenarios: navigation, script evaluation, DOM queries, element interaction (clicks, typing), waiting for elements/events, taking screenshots.
    *   Verify expected browser state, page content, script results, and received events against the live browser.
    *   Run tests against all supported browsers (Chrome, Firefox, Edge) via configuration or test matrix.
    *   Include tests for error conditions (e.g., navigating to invalid URL, command timeouts, interacting with detached elements).
    *   Basic stress tests (e.g., opening many pages, rapid commands) and capture basic performance metrics (e.g., time to navigate, time to execute script).

## 7. Documentation

*   **User Documentation (in `/docs` or website):**
    *   **Getting Started:** Installation, basic usage, launching browsers (`janus-client` API).
    *   **Configuration:** Detailed explanation of `janus.toml` / environment variables (`janus-core` config).
    *   **API Reference:** High-level overview and examples for L1 traits (`janus-interfaces::Browser`, `janus-interfaces::Page`) and common types.
    *   **Browser Specifics:** Notes on setup, configuration, or known quirks for Chrome, Firefox, Edge.
    *   **Examples:** Practical code examples demonstrating common tasks.
    *   **(Future) Plugin Guide:** How to write and use plugins.
*   **Developer Documentation (in `/docs/design`, crate READMEs, code comments):**
    *   **Architecture:** This document (`Janus_Architecture.md`).
    *   **Crate Structure:** Overview of the workspace crates and their responsibilities.
    *   **Actor System:** Detailed explanation of key actors, message flows, supervision (from this doc).
    *   **Protocol Handling:** Specifics of CDP/BiDi mapping in `janus-protocol-handler` and `janus-browser-*`.
    *   **Contribution Guide:** Setup, coding style, testing procedures, PR process.
    *   **Testing Strategy:** Details on running unit, integration, and E2E tests.
*   **API Documentation (generated by `cargo doc`):**
    *   Auto-generated Rust documentation for all public modules, traits, structs, enums, and functions across all crates.
    *   Focus on clear doc comments for `janus-interfaces` (L1 API) and `janus-client` (entry points).
    *   Document public configuration structs in `janus-core`.
    *   Include internal documentation for key components like actors and transport traits.

## 8. Future Considerations

*   **Performance:** Actor mailbox optimization (batching?), zero-copy message parsing (e.g., using `serde_json::RawValue`), optimizing command/event serialization, investigating potential async runtime tuning. Connection pooling for protocols that support it.
*   **Security:** Handling authentication tokens/capabilities (WebDriver BiDi), secure WebSocket connections (WSS), TLS configuration for TCP, considering sandboxing for plugins or script execution contexts. Validating input parameters thoroughly.
*   **Extensibility:** Formalizing the plugin API (`janus-plugin-api`) with clear lifecycle hooks and capabilities. Creating a smoother process/template for adding support for new browsers or debugging protocols. Supporting custom transport implementations.
*   **Advanced Features:** Network request/response interception and modification, HAR file generation, advanced debugging support (breakpoints, stepping, call stacks), code coverage data collection, handling nested browsing contexts (iframes), WebAuthn support.
*   **Ergonomics:** Fluent API design for common actions (chaining), simplifying event handling (e.g., providing stream-based APIs), better error diagnostics.
*   **Distribution:** Packaging, cross-platform compilation considerations, publishing crates to crates.io.

## 9. Version Roadmap (Tentative)

*   **v0.1.0 - Foundation:** Phase 1 complete. Workspace setup, core types, config, basic WebSocket transport, core actor skeletons. Can connect but limited functionality.
*   **v0.2.0 - Chrome MVP:** Phase 2 complete. Functional `CommandActor`, `EventActor`. Basic Chrome CDP implementation via `janus-browser-chrome`. Can navigate, run scripts, close pages/browser via Chrome. Basic E2E tests pass.
*   **v0.3.0 - Chrome Feature Complete & Robust:** Phase 3 complete. Full L1 API implementation for Chrome. Robust error handling and supervision. Comprehensive testing (unit, integration, E2E for Chrome). Basic monitoring capabilities. User-subscribable events.
*   **v0.4.0 - Firefox Support (BiDi):** Phase 4 progress. Add `janus-browser-firefox` crate. Implement L1 API using WebDriver BiDi. Add E2E tests for Firefox.
*   **v0.5.0 - Edge & Plugins:** Phase 4 progress. Add Edge support (leveraging CDP work). Design and implement initial `janus-plugin-api` and `PluginActor` infrastructure.
*   **v1.0.0 - Production Ready:** All major phases complete. Stable support for Chrome, Firefox, Edge. Well-tested, documented, and reasonably performant. Core API is stable.
