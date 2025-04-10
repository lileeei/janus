use serde::{Deserialize, Serialize};

/// Represents a unique identifier for event subscriptions.
pub type SubscriptionId = u64;

/// Represents a handle to an element in the DOM.
/// Specific implementations will hold protocol-specific details internally.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ElementHandle {
    // For now, just a placeholder. Might contain an internal ID or description.
    // The exact structure might evolve based on implementation needs.
    pub description: String,
    // Potentially add remote object ID if common across protocols?
    // pub internal_id: String,
}

/// Represents a message logged to the browser's console.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConsoleMessage {
    pub level: ConsoleLogLevel,
    pub text: String,
    // Potentially add source (JS, network, etc.), line number, args etc.
}

/// Severity level of a console message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConsoleLogLevel {
    Log,
    Debug,
    Info,
    Warning,
    Error,
    // Other levels like trace, table, etc. might be added
}

/// Available formats for taking screenshots.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ScreenshotFormat {
    Png,
    Jpeg,
    // WebP might be added later
}

/// Options for taking a screenshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ScreenshotOptions {
    /// Capture the screenshot from the surface, rather than the view. Defaults to true.
    pub capture_beyond_viewport: Option<bool>,
    /// Specify a specific area to capture.
    pub clip: Option<Clip>,
    /// Format of the resulting image. Defaults to Png.
    pub format: Option<ScreenshotFormat>,
    /// Quality of the image (0-100). Only applicable to Jpeg.
    pub quality: Option<u8>,
    /// When true, encodes the screenshot in base64. Defaults to false (returns raw bytes).
    /// Note: The trait returns `Vec<u8>`, so users might need to encode/decode if using this.
    pub from_surface: Option<bool>,
    // Optimize for speed? (CDP specific?)
}

/// Specifies a rectangular area.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Clip {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
