//! A simplified example of using Janus to connect to Chrome

use janus_client::{ApiError, Browser, Page};
use janus_client::launch::{self, LaunchMode};
use serde_json::Value;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), ApiError> {
    println!("Starting Chrome example...");
    // Connect to Chrome running with --remote-debugging-port=9222
    let launch_mode = LaunchMode::Connect {
        url: "ws://127.0.0.1:9222".to_string(),
    };
    
    let mut browser = launch::launch(launch_mode, None).await
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