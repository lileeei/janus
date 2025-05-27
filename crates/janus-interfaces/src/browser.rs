use crate::error::ApiError;
use crate::page::Page;
use std::fmt::Debug;

/// Represents a connection to and control over a web browser instance.
///
/// This trait provides a high-level, protocol-agnostic API for interacting
/// with the browser, managing pages/targets, and accessing browser-level information.
// We need to use different approaches for Rust traits with async methods
// This trait defines the interface, but we'll need wrappers for dyn usage
#[async_trait::async_trait]
pub trait Browser: Send + Sync + Debug {
    // Note: Connecting is typically handled by a separate launch/connect function
    // that returns `Result<impl Browser, ApiError>`, rather than being a method
    // on the trait itself after instantiation.

    /// Disconnects the client from the browser's debugging protocol.
    /// The browser process itself might remain running.
    ///
    /// # Returns
    /// - `Ok(())` on successful disconnection.
    /// - `Err(ApiError)` if disconnection fails or was not connected.
    async fn disconnect(&mut self) -> Result<(), ApiError>;

    /// Closes the browser instance by sending the appropriate command (e.g., `Browser.close` via CDP)
    /// to the browser, which should terminate the browser process.
    /// This implicitly disconnects the client if the browser closes successfully.
    ///
    /// # Returns
    /// - `Ok(())` on successful initiation of the close command and local actor shutdown.
    /// - `Err(ApiError)` if sending the close command fails or the local actor system encounters an error.
    async fn close(&mut self) -> Result<(), ApiError>;

    /// Creates a new browser tab or page (target).
    ///
    /// # Returns
    /// - `Ok(Box<dyn Page>)` containing a handle to the newly created page.
    /// - `Err(ApiError)` if creating the page fails.
    async fn new_page(&self) -> Result<Box<dyn Page>, ApiError>;

    /// Retrieves handles to all currently open pages/tabs (targets) within the browser.
    ///
    /// # Returns
    /// - `Ok(Vec<Box<dyn Page>>)` containing handles to the pages.
    /// - `Err(ApiError)` if retrieving the page list fails.
    async fn pages(&self) -> Result<Vec<Box<dyn Page>>, ApiError>;

    /// Gets version information about the browser.
    ///
    /// # Returns
    /// - `Ok(String)` containing browser version details (format may vary).
    /// - `Err(ApiError)` if fetching version information fails.
    async fn version(&self) -> Result<String, ApiError>;

    /// Resets all browser permissions, either for the entire browser or for a specific
    /// browser context (if supported by the underlying protocol).
    ///
    /// This function attempts to send a command like `Browser.resetPermissions` via CDP.
    ///
    /// # Arguments
    /// - `browser_context_id`: An optional `String` identifying a specific browser context
    ///                         (e.g., a profile or incognito session). If `None`, permissions
    ///                         are reset globally for the browser. The interpretation of this ID
    ///                         is protocol-specific.
    ///
    /// # Returns
    /// - `Ok(())` if the command to reset permissions was successfully sent and acknowledged.
    /// - `Err(ApiError)` if sending the command fails, the browser indicates an error,
    ///   or other issues occur (e.g., serialization, actor communication).
    async fn reset_permissions(&mut self, browser_context_id: Option<String>) -> Result<(), ApiError>;

    // --- Event Subscription (Placeholder - Requires careful design) ---
    // Event subscription APIs might return stream handles or require callbacks.
    // This is a complex area deferred beyond Phase 1.

    // Example concept (details TBD):
    // async fn on_target_created(&self, handler: Box<dyn Fn(Box<dyn Page>) + Send + Sync + 'static>) -> Result<SubscriptionId, ApiError>;
    // async fn unsubscribe(&self, id: SubscriptionId) -> Result<(), ApiError>;
}
