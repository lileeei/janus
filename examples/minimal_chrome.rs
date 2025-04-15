//! A minimal Chrome example

use janus_client::launch::LaunchMode;
use janus_client::{ApiError, launch};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), ApiError> {
    // Connect to Chrome running on default port
    // Make sure to start Chrome with: chrome --remote-debugging-port=9222
    let launch_mode = LaunchMode::Connect {
        url: "ws://127.0.0.1:9222/devtools/browser".to_string(),
    };
    
    let mut browser = launch(launch_mode, None).await
        .map_err(|e| ApiError::LaunchError(format!("Failed to connect: {}", e)))?;
    
    println!("Successfully connected to browser");
    
    // Get browser version
    let version = browser.version().await?;
    println!("Browser version: {}", version);
    
    // Create a new page
    let page = browser.new_page().await?;
    println!("Created new page with ID: {}", page.id());
    
    // Navigate to a website
    page.navigate("https://example.com").await?;
    println!("Navigated to example.com");
    
    // Wait for page to load
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Get page title
    let title = page.title().await?;
    println!("Page title: {}", title);
    
    // Clean up
    page.close().await?;
    browser.disconnect().await?;
    
    println!("Example completed successfully");
    Ok(())
}