//! Browser launching logic.

use crate::error::ClientError;
use crate::supervisor::{CoreActorsInfo, StartBrowserActor, StartCoreActors, SupervisorActor};
use janus_browser_chrome::ChromeBrowser; // Import L2 implementation
use janus_core::config::{self, BrowserLaunchConfig, Config};
use janus_core::logging;
use janus_interfaces::{ApiError, Browser}; // Use L1 traits
use janus_transport::ConnectParams;

use actix::prelude::*;
use log::{debug, error, info, warn};

/// Specifies how to start a browser session.
#[derive(Debug, Clone)]
pub enum LaunchMode {
    /// Connect to an existing browser instance at the given debugging URL.
    Connect { url: String },
    /// Launch a new browser instance using configuration.
    Launch {
        /// Optional identifier ("chrome", "firefox") to load specific config from janus.toml `[browsers.<id>]` table.
        /// If None, uses `[browser_defaults]`.
        browser_id: Option<String>,
        /// Optional overrides for the loaded configuration.
        overrides: Option<BrowserLaunchConfig>,
    },
}

/// Launches or connects to a browser based on options and loaded configuration.
///
/// This is the primary entry point for starting a Janus session.
///
/// # Arguments
/// * `mode` - Specifies whether to launch a new instance or connect to an existing one.
/// * `config` - Optional pre-loaded configuration. If None, calls `load_config()`.
///
/// # Returns
/// A result containing a `Box<dyn Browser>` handle or a `ClientError`.
pub async fn launch(
    mode: LaunchMode,
    config: Option<Config>,
) -> Result<Box<dyn Browser>, ClientError> {
    // 1. Load configuration if not provided
    let cfg = match config {
        Some(c) => c,
        None => config::load_config().map_err(ClientError::ConfigError)?,
    };

    // 2. Setup logging
    if let Err(e) = logging::setup_logging(&cfg.global.log_level) {
        eprintln!("Warning: Failed to initialize logging: {}", e);
    }

    info!("Janus Client starting...");
    debug!("Loaded configuration: {:?}", cfg); // Be careful logging sensitive config

    // 3. Determine ConnectParams and Launch specific config
    // TODO (Phase 3): Implement actual browser process launching in determine_connection
    let (connect_params, _launch_config) =
        determine_connection_params(&mode, &cfg).await?; // launch_config needed later for process mgmt

    // --- Phase 2: Actor System and Wiring ---

    // 4. Start Actor System and Supervisor
    info!("Starting Janus actor system...");
    // Ensure a system exists. Starting multiple systems might cause issues depending on setup.
    // Consider a global system or managing it per launch instance carefully.
    if System::current_opt().is_none() {
        info!("No running Actix system found, starting a new one.");
        // Starting a system like this makes it hard to reuse across multiple launches.
        // A better pattern might be needed for library usage.
        // For now, let launch start a new one if needed.
         System::new(); // Creates and sets as current
    } else {
         info!("Using existing Actix system.");
    }


    // Start the main supervisor actor
    let supervisor_addr = SupervisorActor::new(cfg.clone()).start();
    info!("SupervisorActor started at Addr: {:?}", supervisor_addr);

    // 5. Supervisor launches core actors (Connection, Command, Event)
    let core_actors_info: CoreActorsInfo = supervisor_addr
        .send(StartCoreActors(connect_params.clone())) // Clone params
        .await
        .map_err(|mb_err| ClientError::SupervisorError(format!("Mailbox error starting core actors: {}", mb_err)))? // Mailbox error
        .map_err(|internal_err| ClientError::SupervisorError(format!("Failed to start core actors: {}", internal_err)))?; // Logical error

    info!("Core actors started successfully.");
    // TODO: Wait for connection to be established? Supervisor should handle this maybe.
    // For now, assume connection will establish or fail shortly after.

    // 6. Supervisor launches the appropriate BrowserActor (e.g., ChromeBrowserActor)
    // Determine browser type based on launch_config or connection URL?
    // For Phase 2, assume Chrome.
    let browser_actor_addr = supervisor_addr
        .send(StartBrowserActor { core_actors: core_actors_info })
        .await
        .map_err(|mb_err| ClientError::SupervisorError(format!("Mailbox error starting browser actor: {}", mb_err)))?
        .map_err(|internal_err| ClientError::LaunchError(format!("Failed to start browser actor: {}", internal_err)))?;

    info!("ChromeBrowserActor started successfully.");
    // TODO: Wait for BrowserActor to signal readiness?

    // 7. Create the L2 Browser implementation (e.g., ChromeBrowser)
    // Give it the Addr of the BrowserActor
    let browser_impl = ChromeBrowser::new(browser_actor_addr);

    // 8. Return the L2 implementation boxed as `dyn Browser`
    info!("Janus client launch sequence complete.");
    Ok(Box::new(browser_impl))
}

/// Determines the connection parameters based on launch mode and config.
/// Phase 2: Does *not* actually launch the browser process yet.
async fn determine_connection_params(
    mode: &LaunchMode,
    cfg: &Config,
) -> Result<(ConnectParams, BrowserLaunchConfig), ClientError> {
    match mode {
        LaunchMode::Connect { url } => {
            info!("Connecting to existing browser at: {}", url);
            let params = ConnectParams {
                url: url.clone(),
                connection_timeout: cfg.transport.connect_timeout,
                #[cfg(feature = "websocket")]
                ws_options: cfg.transport.websocket.clone(),
            };
            Ok((params, BrowserLaunchConfig::default())) // No launch config needed
        }
        LaunchMode::Launch {
            browser_id,
            overrides,
        } => {
            info!("Preparing connection for launching new browser instance (id: {:?})", browser_id);
            let base_config = browser_id
                .as_ref()
                .and_then(|id| cfg.browsers.get(id))
                .unwrap_or(&cfg.browser_defaults);

            let launch_cfg = overrides
                .as_ref()
                .map(|ovr| ovr.merged_with(base_config))
                .unwrap_or_else(|| base_config.clone());

            debug!("Effective launch configuration: {:?}", launch_cfg);

            // Phase 2: Determine connection URL *without* launching process
            let url = if let Some(override_url) = &launch_cfg.connection_url_override {
                info!("Using connection URL override: {}", override_url);
                override_url.clone()
            } else {
                // Construct URL from config defaults (e.g., localhost:9222)
                // This relies on the user manually starting Chrome with remote debugging enabled for now.
                let port = launch_cfg.remote_debugging_port.unwrap_or(9222);
                let addr = launch_cfg
                    .remote_debugging_address
                    .as_deref()
                    .unwrap_or("127.0.0.1");
                let default_url = format!("ws://{}:{}", addr, port); // Basic WS URL
                warn!(
                    "Phase 2: Browser process launching not implemented. Assuming browser is running at {}", default_url
                );
                // TODO (Phase 3): Need to fetch the actual devtools endpoint, often includes /devtools/browser/UUID
                 warn!("Phase 2: Using base URL '{}'. Actual connection might require a specific path like /devtools/browser/...", default_url);
                 // Returning the base URL. Connection might fail if specific endpoint needed.
                 default_url
                 // Better placeholder for Phase 2 testing: fixed known endpoint from manually launched Chrome
                // format!("ws://{}:{}/devtools/browser/...", addr, port) // Replace ... with actual UUID if known

                 // Let's use a known default that often works if Chrome launched simply
                 // format!("ws://{}:{}/devtools/browser", addr, port)
            };


            let params = ConnectParams {
                url,
                connection_timeout: cfg.transport.connect_timeout,
                #[cfg(feature = "websocket")]
                ws_options: cfg.transport.websocket.clone(), // TODO: Merge from launch_cfg if needed
            };

            Ok((params, launch_cfg))
        }
    }
}
