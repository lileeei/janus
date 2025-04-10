//! L2 Implementation of `janus_interfaces::Page` for Chrome.

use crate::actors::{ChromePageActor, ClosePage, EvaluateScript, Navigate};
use crate::error::map_internal_to_api_error; // Need error mapping
use actix::prelude::*;
use async_trait::async_trait;
use janus_interfaces::{
    ApiError, ElementHandle, Page, ScreenshotFormat, ScreenshotOptions, Value,
};
use log::debug;


// Represents a handle to a specific Chrome page/target
pub struct ChromePage {
    pub(crate) actor_addr: Addr<ChromePageActor>,
    page_id: String, // Store the ID for the id() method
}

impl ChromePage {
    pub(crate) fn new(actor_addr: Addr<ChromePageActor>, page_id: String) -> Self {
        Self { actor_addr, page_id }
    }
}

#[async_trait]
impl Page for ChromePage {
    async fn navigate(&self, url: &str) -> Result<(), ApiError> {
        debug!("ChromePage ({})::navigate requested to URL: {}", self.page_id, url);
        self.actor_addr.send(Navigate { url: url.to_string() })
            .await
            .map_err(|mb_err| ApiError::InternalError(format!("Mailbox error navigating: {}", mb_err)))?
            .map_err(map_internal_to_api_error)
    }

    async fn reload(&self) -> Result<(), ApiError> {
        debug!("ChromePage ({})::reload requested.", self.page_id);
        // Send Page.reload command
        let cmd = json!({}); // Page.reload takes optional args like ignoreCache
        self.actor_addr
            .send(EvaluateScript { script: "location.reload()".to_string() }) // Simplification
            // TODO: Send actual Page.reload command via actor
            .await
            .map_err(|mb_err| ApiError::InternalError(format!("Mailbox error reloading: {}", mb_err)))?
            .map_err(map_internal_to_api_error)?;
        Ok(())
    }

    async fn go_back(&self) -> Result<(), ApiError> {
        debug!("ChromePage ({})::go_back requested.", self.page_id);
        self.actor_addr
             .send(EvaluateScript { script: "history.back()".to_string() }) // Simplification
             // TODO: Send actual Page.goBack command via actor
             .await
             .map_err(|mb_err| ApiError::InternalError(format!("Mailbox error going back: {}", mb_err)))?
             .map_err(map_internal_to_api_error)?;
         Ok(())
    }

     async fn go_forward(&self) -> Result<(), ApiError> {
         debug!("ChromePage ({})::go_forward requested.", self.page_id);
         self.actor_addr
             .send(EvaluateScript { script: "history.forward()".to_string() }) // Simplification
             // TODO: Send actual Page.goForward command via actor
             .await
             .map_err(|mb_err| ApiError::InternalError(format!("Mailbox error going forward: {}", mb_err)))?
             .map_err(map_internal_to_api_error)?;
         Ok(())
     }

    async fn close(&self) -> Result<(), ApiError> {
        debug!("ChromePage ({})::close requested.", self.page_id);
        self.actor_addr.send(ClosePage)
            .await
            .map_err(|mb_err| ApiError::InternalError(format!("Mailbox error closing page: {}", mb_err)))?
            .map_err(map_internal_to_api_error)
    }

    fn id(&self) -> String {
        self.page_id.clone()
    }

    async fn content(&self) -> Result<String, ApiError> {
        debug!("ChromePage ({})::content requested.", self.page_id);
        // Use Runtime.evaluate to get document.documentElement.outerHTML
        let script = "document.documentElement.outerHTML".to_string();
        let result = self.actor_addr.send(EvaluateScript { script })
            .await
            .map_err(|mb_err| ApiError::InternalError(format!("Mailbox error getting content: {}", mb_err)))?
            .map_err(map_internal_to_api_error)?;

        result.as_str().map(String::from).ok_or_else(|| {
            ApiError::InternalError("Failed to get string content from evaluation".to_string())
        })
    }

    async fn evaluate_script(&self, script: &str) -> Result<Value, ApiError> {
        debug!("ChromePage ({})::evaluate_script requested.", self.page_id);
        self.actor_addr.send(EvaluateScript { script: script.to_string() })
            .await
            .map_err(|mb_err| ApiError::InternalError(format!("Mailbox error evaluating script: {}", mb_err)))?
            .map_err(map_internal_to_api_error)
    }

    // --- Methods below are placeholders for Phase 2 ---

    async fn call_function(
        &self,
        _function_declaration: &str,
        _args: Vec<Value>,
    ) -> Result<Value, ApiError> {
        warn!("ChromePage::call_function not implemented yet.");
        Err(ApiError::NotSupported("call_function".to_string()))
    }

    async fn query_selector(&self, _selector: &str) -> Result<Option<ElementHandle>, ApiError> {
        warn!("ChromePage::query_selector not implemented yet.");
        Err(ApiError::NotSupported("query_selector".to_string()))
        // Implementation: Send DOM.querySelector command, parse result (NodeId), create ElementHandle
    }

    async fn wait_for_selector(
        &self,
        _selector: &str,
        _timeout_ms: u64,
    ) -> Result<ElementHandle, ApiError> {
        warn!("ChromePage::wait_for_selector not implemented yet.");
        Err(ApiError::NotSupported("wait_for_selector".to_string()))
        // Implementation: Combine polling evaluate_script or DOM mutation observers
    }

    async fn url(&self) -> Result<String, ApiError> {
        warn!("ChromePage::url not implemented yet.");
         // Use Runtime.evaluate 'window.location.href'
        let script = "window.location.href".to_string();
        let result = self.actor_addr.send(EvaluateScript { script })
            .await
            .map_err(|mb_err| ApiError::InternalError(format!("Mailbox error getting url: {}", mb_err)))?
            .map_err(map_internal_to_api_error)?;
         result.as_str().map(String::from).ok_or_else(|| {
            ApiError::InternalError("Failed to get string url from evaluation".to_string())
        })
    }

    async fn title(&self) -> Result<String, ApiError> {
        warn!("ChromePage::title not implemented yet.");
        // Use Runtime.evaluate 'document.title'
        let script = "document.title".to_string();
         let result = self.actor_addr.send(EvaluateScript { script })
            .await
            .map_err(|mb_err| ApiError::InternalError(format!("Mailbox error getting title: {}", mb_err)))?
            .map_err(map_internal_to_api_error)?;
         result.as_str().map(String::from).ok_or_else(|| {
            ApiError::InternalError("Failed to get string title from evaluation".to_string())
        })
    }

    async fn take_screenshot(
        &self,
        _format: ScreenshotFormat,
        _options: ScreenshotOptions,
    ) -> Result<Vec<u8>, ApiError> {
        warn!("ChromePage::take_screenshot not implemented yet.");
        Err(ApiError::NotSupported("take_screenshot".to_string()))
        // Implementation: Send Page.captureScreenshot command
    }
}
