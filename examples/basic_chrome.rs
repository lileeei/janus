//! A basic example of using Janus to connect to Chrome

use janus_client::{ApiError, Browser, Page, launch};
use janus_client::launch::LaunchMode;
use serde_json::Value;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), ApiError> {
    // Connect to an existing Chrome browser instance
    // Note: You need to start Chrome with remote debugging enabled:
    // chrome --remote-debugging-port=9222
    
    let launch_mode = LaunchMode::Connect {
        url: "ws://127.0.0.1:9222/devtools/browser/c4fc01c2-a1ac-4a5c-a9bc-a39f5d88583a".to_string(),
    };
    
    let mut browser = launch(launch_mode, None).await
        .map_err(|e| ApiError::LaunchError(format!("Failed to launch browser: {}", e)))?;
    
    println!("Connected to browser. Getting version...");
    let version = browser.version().await?;
    println!("Browser version: {}", version);
    
    // Create a new page
    println!("Creating a new page...");
    let page = browser.new_page().await?;
    
    // Navigate to a website
    println!("Navigating to example.com...");
    page.navigate("https://example.com").await?;
    
    // Wait a bit for the page to load
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    // Get the page title
    let title = page.title().await?;
    println!("Page title: {}", title);
    
    // Get the page content
    let content = page.content().await?;
    println!("Page content (first 100 chars): {}...", &content[..100.min(content.len())]);
    
    // Execute some JavaScript
    println!("Executing JavaScript...");
    let result: Value = page.evaluate_script(
        "document.querySelector('h1').textContent"
    ).await?;
    println!("H1 content: {}", result);
    
    // Close the page
    println!("Closing page...");
    page.close().await?;
    
    // Disconnect from the browser
    println!("Disconnecting from browser...");
    browser.disconnect().await?;
    
    println!("Example completed successfully!");
    Ok(())
}