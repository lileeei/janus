# Janus - Unified Browser Debugging Client

Janus is a Rust client library that provides a unified interface for browser automation and debugging across different browser engines and debugging protocols. It supports Chrome DevTools Protocol (CDP) and aims to add support for WebDriver BiDi.

## Project Structure

Janus follows a layered architecture:

- **L1**: Protocol-agnostic API (`Browser`, `Page` traits)
- **L2**: Browser-specific implementations (Chrome, Firefox)
- **L3**: Transport layer (WebSocket, TCP)

The codebase uses an actor-based concurrency model (via Actix) for handling communication with browsers.

## Current Status

The project currently implements:

- Basic Chrome support via CDP
- WebSocket transport layer
- Actor-based messaging system
- Page navigation, script evaluation, and basic operations

## Prerequisites

- Rust 2024 edition or later
- Google Chrome with remote debugging enabled

## Running the Examples

### 1. Start Chrome with Remote Debugging

```bash
# For Chrome/Chromium on macOS/Linux
chrome --remote-debugging-port=9222 --headless=new

# For Chrome on Windows
start chrome.exe --remote-debugging-port=9222 --headless=new
```

### 2. Run the Simple Example

```bash
cargo run --example minimal_chrome
```

This example connects to Chrome, creates a new page, navigates to example.com, and fetches the page title.

### 3. More Examples

```bash
# More comprehensive demo
cargo run --example chrome_demo
```

## Sample Usage

```rust
use janus_client::{ApiError, Browser, Page, launch};
use janus_client::launch::LaunchMode;

#[tokio::main]
async fn main() -> Result<(), ApiError> {
    // Connect to Chrome running with remote debugging
    let launch_mode = LaunchMode::Connect {
        url: "ws://127.0.0.1:9222/devtools/browser".to_string(),
    };
    
    let mut browser = launch(launch_mode, None).await?;
    println!("Connected to browser");
    
    // Create a new page
    let page = browser.new_page().await?;
    
    // Navigate to a website
    page.navigate("https://example.com").await?;
    
    // Get page title
    let title = page.title().await?;
    println!("Page title: {}", title);
    
    // Execute JavaScript
    let heading = page.evaluate_script("document.querySelector('h1').textContent").await?;
    println!("Heading: {}", heading);
    
    // Cleanup
    // Closing the page first (optional, as browser.close() might handle this)
    page.close().await?; 
    // Close the browser (which also terminates the process and disconnects)
    browser.close().await?;
    
    Ok(())
}
```

## Project Status

Janus is currently a work in progress with basic Chrome support. Key functionalities include:
- Connecting to a running Chrome instance.
- Creating new pages (tabs).
- Navigating pages to URLs.
- Evaluating JavaScript on pages.
- Getting page titles.
- Closing the browser instance (terminating the process).
- Resetting browser permissions (e.g., for specific contexts).

The project aims to add:

- Firefox support via WebDriver BiDi
- Browser process launching
- More advanced page operations (screenshots, element handling, etc.)
- Plugin system

## Contributing

Contributions are welcome! Please see the architecture documentation in the `docs/` directory for more information on the design and implementation details.