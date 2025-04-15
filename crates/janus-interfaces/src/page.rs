use crate::common::*;
use crate::error::ApiError;
use async_trait::async_trait;
use serde_json::Value;
use std::fmt::Debug;

/// Represents a single browser page, tab, or other target (like a WebWorker).
///
/// This trait provides a high-level, protocol-agnostic API for interacting
/// with the content and state of a specific target.
#[async_trait::async_trait]
pub trait Page: Send + Sync + Debug {
    /// Navigates the page to the specified URL.
    ///
    /// # Arguments
    /// * `url` - The URL to navigate to.
    ///
    /// # Returns
    /// - `Ok(())` on successful navigation initiation (doesn't necessarily wait for load).
    /// - `Err(ApiError)` if navigation fails (e.g., invalid URL, network error).
    async fn navigate(&self, url: &str) -> Result<(), ApiError>;

    /// Reloads the current page.
    ///
    /// # Returns
    /// - `Ok(())` on successful reload initiation.
    /// - `Err(ApiError)` if reloading fails.
    async fn reload(&self) -> Result<(), ApiError>;

    /// Navigates the page back in its history.
    ///
    /// # Returns
    /// - `Ok(())` if navigation back is possible and initiated.
    /// - `Err(ApiError)` if navigation fails (e.g., no history).
    async fn go_back(&self) -> Result<(), ApiError>;

    /// Navigates the page forward in its history.
    ///
    /// # Returns
    /// - `Ok(())` if navigation forward is possible and initiated.
    /// - `Err(ApiError)` if navigation fails (e.g., no history).
    async fn go_forward(&self) -> Result<(), ApiError>;

    /// Closes this specific page or target.
    ///
    /// # Returns
    /// - `Ok(())` on successful closure.
    /// - `Err(ApiError)` if closing fails.
    async fn close(&self) -> Result<(), ApiError>;

    /// Returns the unique identifier for this page/target within the browser session.
    /// The format might be protocol-specific (e.g., TargetID in CDP).
    fn id(&self) -> String;

    /// Gets the full HTML content of the page.
    ///
    /// # Returns
    /// - `Ok(String)` containing the page's HTML.
    /// - `Err(ApiError)` if retrieving content fails.
    async fn content(&self) -> Result<String, ApiError>;

    /// Evaluates a JavaScript expression in the context of the page.
    ///
    /// # Arguments
    /// * `script` - The JavaScript code to evaluate.
    ///
    /// # Returns
    /// - `Ok(serde_json::Value)` representing the result of the expression.
    /// - `Err(ApiError)` if evaluation fails (e.g., script error, serialization issue).
    async fn evaluate_script(&self, script: &str) -> Result<Value, ApiError>;

    /// Calls a JavaScript function defined in the page context.
    ///
    /// # Arguments
    /// * `function_declaration` - A string containing the function declaration (e.g., `function(a, b) { return a + b; }`).
    /// * `args` - A vector of JSON values to pass as arguments to the function.
    ///
    /// # Returns
    /// - `Ok(serde_json::Value)` representing the function's return value.
    /// - `Err(ApiError)` if calling the function fails.
    async fn call_function(
        &self,
        function_declaration: &str,
        args: Vec<Value>,
    ) -> Result<Value, ApiError>;

    /// Finds the first element matching the given CSS selector.
    ///
    /// # Arguments
    /// * `selector` - The CSS selector to query for.
    ///
    /// # Returns
    /// - `Ok(Some(ElementHandle))` if an element is found.
    /// - `Ok(None)` if no element matches the selector.
    /// - `Err(ApiError)` if the query fails.
    async fn query_selector(&self, selector: &str) -> Result<Option<ElementHandle>, ApiError>;

    /// Waits for an element matching the selector to appear in the DOM.
    ///
    /// # Arguments
    /// * `selector` - The CSS selector to wait for.
    /// * `timeout_ms` - Maximum time in milliseconds to wait.
    ///
    /// # Returns
    /// - `Ok(ElementHandle)` when the element is found within the timeout.
    /// - `Err(ApiError::Timeout)` if the timeout is reached before the element is found.
    /// - `Err(ApiError)` for other failures.
    async fn wait_for_selector(
        &self,
        selector: &str,
        timeout_ms: u64,
    ) -> Result<ElementHandle, ApiError>;

    /// Gets the current URL of the page.
    ///
    /// # Returns
    /// - `Ok(String)` containing the URL.
    /// - `Err(ApiError)` if retrieving the URL fails.
    async fn url(&self) -> Result<String, ApiError>;

    /// Gets the title of the page.
    ///
    /// # Returns
    /// - `Ok(String)` containing the title.
    /// - `Err(ApiError)` if retrieving the title fails.
    async fn title(&self) -> Result<String, ApiError>;

    /// Takes a screenshot of the current page viewport or a specified area.
    ///
    /// # Arguments
    /// * `format` - The desired image format (Png, Jpeg).
    /// * `options` - Additional options for the screenshot (quality, clip, etc.).
    ///
    /// # Returns
    /// - `Ok(Vec<u8>)` containing the raw image bytes in the specified format.
    /// - `Err(ApiError)` if taking the screenshot fails.
    async fn take_screenshot(
        &self,
        format: ScreenshotFormat,
        options: ScreenshotOptions,
    ) -> Result<Vec<u8>, ApiError>;

    // --- Input Methods (Placeholder - Defined but not implemented in Phase 1) ---
    // async fn click(&self, selector: &str) -> Result<(), ApiError>;
    // async fn type_text(&self, selector: &str, text: &str) -> Result<(), ApiError>;

    // --- Event Subscription (Placeholder - Complex, deferred beyond Phase 1) ---
    // async fn on_load(&self, handler: Box<dyn Fn() + Send + Sync + 'static>) -> Result<SubscriptionId, ApiError>;
    // async fn on_console_message(&self, handler: Box<dyn Fn(ConsoleMessage) + Send + Sync + 'static>) -> Result<SubscriptionId, ApiError>;
}
