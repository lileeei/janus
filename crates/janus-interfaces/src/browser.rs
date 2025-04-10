use crate::common::*;
use crate::error::ApiError;
use crate::page::Page;
use async_trait::async_trait;

/// Represents a connection to and control over a web browser instance.
///
/// This trait provides a high-level, protocol-agnostic API for interacting
/// with the browser, managing pages/targets, and accessing browser-level information.
#[async_trait]
pub trait Browser: Send + Sync {
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

    /// Closes the browser instance, terminating the browser process.
    /// This implicitly disconnects the client.
    ///
    /// # Returns
    /// - `Ok(())` on successful closure.
    /// - `Err(ApiError)` if closing fails (e.g., process not running, permissions).
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

    // --- Event Subscription (Placeholder - Requires careful design) ---
    // Event subscription APIs might return stream handles or require callbacks.
    // This is a complex area deferred beyond Phase 1.

    // Example concept (details TBD):
    // async fn on_target_created(&self, handler: Box<dyn Fn(Box<dyn Page>) + Send + Sync + 'static>) -> Result<SubscriptionId, ApiError>;
    // async fn unsubscribe(&self, id: SubscriptionId) -> Result<(), ApiError>;
}
