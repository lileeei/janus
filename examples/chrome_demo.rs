//! Demo of using Janus to automate Chrome
//!
//! This example requires Chrome to be running with remote debugging enabled:
//! `chrome --remote-debugging-port=9222 --headless=new`

use janus_client::{ApiError, Browser, Page};
use janus_client::launch::{self, LaunchMode};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), ApiError> {
    println!("Janus Chrome Demo - Connecting to browser");
    
    // Connect to an existing Chrome browser running with remote debugging
    let launch_mode = LaunchMode::Launch {
        browser_id: Some("chrome".to_string()),
        overrides: None,
    };
    
    let mut browser = launch::launch(launch_mode, None).await
        .map_err(|e| ApiError::LaunchError(format!("Failed to connect to browser: {}", e)))?;
    
    println!("Connected to browser successfully");
    let version = browser.version().await?;
    println!("Browser version: {}", version);
    
    // Create a new page
    println!("Opening a new page");
    let page = browser.new_page().await?;
    
    // Navigate to a website
    println!("Navigating to example.com");
    page.navigate("https://example.com").await?;
    
    // Wait a bit for the page to load completely
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Get the page title
    let title = page.title().await?;
    println!("Page title: {}", title);
    
    // Get the heading content using JavaScript
    let heading = page.evaluate_script("document.querySelector('h1').textContent.trim()").await?;
    println!("Page heading: {}", heading);
    
    // Get all paragraph text
    let paragraphs = page.evaluate_script("
        Array.from(document.querySelectorAll('p'))
            .map(p => p.textContent.trim())
            .join('\n')
    ").await?;
    println!("Page paragraphs:\n{}", paragraphs);
    
    // Navigate to another page
    println!("\nNavigating to a second page (mozilla.org)");
    page.navigate("https://www.mozilla.org").await?;
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Get the title
    let mozilla_title = page.title().await?;
    println!("Mozilla page title: {}", mozilla_title);
    
    // Take a screenshot (if implemented)
    // println!("Taking a screenshot");
    // let screenshot = page.take_screenshot(ScreenshotFormat::Png, ScreenshotOptions::default()).await?;
    // println!("Screenshot taken ({} bytes)", screenshot.len());
    
    // Close the page
    println!("Closing page");
    page.close().await?;
    
    // Close the browser
    println!("Disconnecting from browser");
    browser.disconnect().await?;
    
    println!("Demo completed successfully!");
    Ok(())
}