//! L2 Implementation of `janus_interfaces::Browser` for Chrome.

use crate::actors::{ChromeBrowserActor, CreatePage, GetPages, GetVersion, PageInfo, ShutdownBrowser};
use crate::error::map_internal_to_api_error; // Need an error mapping module
use crate::page::ChromePage;
use actix::prelude::*;
use async_trait::async_trait;
use janus_interfaces::{ApiError, Browser, Page};
use log::debug;

// Represents the user-facing handle to a Chrome browser instance
pub struct ChromeBrowser {
    // Internal handle to the actor managing this browser instance.
    actor_addr: Addr<ChromeBrowserActor>,
}

impl ChromeBrowser {
    // Renamed from launch, called by janus-client::launch
    pub(crate) fn new(actor_addr: Addr<ChromeBrowserActor>) -> Self {
        Self { actor_addr }
    }
}

#[async_trait]
impl Browser for ChromeBrowser {
    async fn disconnect(&mut self) -> Result<(), ApiError> {
        // Disconnect usually means stop interacting, potentially stop actors.
        // For CDP, there isn't a specific disconnect command like WebSocket close.
        // Let's interpret this as stopping the BrowserActor.
        debug!("ChromeBrowser::disconnect requested. Stopping BrowserActor.");
        self.actor_addr.send(ShutdownBrowser).await
             .map_err(|mb_err| ApiError::InternalError(format!("Mailbox error stopping browser actor: {}", mb_err)))?;
        Ok(())
    }

    async fn close(&mut self) -> Result<(), ApiError> {
        // Close should terminate the browser process.
        // BrowserActor should handle sending Browser.close or Target.closeTarget commands.
        // For now, it just stops the actor system part.
        debug!("ChromeBrowser::close requested. Sending ShutdownBrowser to BrowserActor.");
        self.actor_addr.send(ShutdownBrowser).await
            .map_err(|mb_err| ApiError::InternalError(format!("Mailbox error closing browser: {}", mb_err)))?;
        // TODO (Phase 3): Ensure Browser.close is actually sent.
        Ok(())
    }

    async fn new_page(&self) -> Result<Box<dyn Page>, ApiError> {
        let url = "about:blank".to_string(); // Default URL for new tabs
        debug!("ChromeBrowser::new_page requested (url: {})", url);
        let response = self.actor_addr
            .send(CreatePage { url })
            .await
            .map_err(|mb_err| ApiError::InternalError(format!("Mailbox error creating page: {}", mb_err)))? // Mailbox Error
            .map_err(map_internal_to_api_error)?; // Logical Error

        Ok(Box::new(ChromePage::new(response.page_actor_addr, response.page_id)))
    }

    async fn pages(&self) -> Result<Vec<Box<dyn Page>>, ApiError> {
         debug!("ChromeBrowser::pages requested.");
         let pages_info: Vec<PageInfo> = self.actor_addr.send(GetPages)
             .await
             .map_err(|mb_err| ApiError::InternalError(format!("Mailbox error getting pages: {}", mb_err)))?
             .map_err(map_internal_to_api_error)?;

         let pages: Vec<Box<dyn Page>> = pages_info
            .into_iter()
            .map(|info| Box::new(ChromePage::new(info.actor_addr, info.id)) as Box<dyn Page>)
            .collect();
         Ok(pages)
    }

    async fn version(&self) -> Result<String, ApiError> {
        debug!("ChromeBrowser::version requested.");
        self.actor_addr.send(GetVersion)
             .await
             .map_err(|mb_err| ApiError::InternalError(format!("Mailbox error getting version: {}", mb_err)))?
             .map_err(map_internal_to_api_error)
    }
}

impl Drop for ChromeBrowser {
    fn drop(&mut self) {
        // Optional: Send a disconnect/shutdown message on drop if not already closed?
        // Be careful about async operations in drop. Best practice is explicit close/disconnect.
        // info!("ChromeBrowser handle dropped.");
        // let addr = self.actor_addr.clone();
        // actix::spawn(async move {
        //     addr.do_send(ShutdownBrowser);
        // });
    }
}
