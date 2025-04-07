use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Serialize)]
pub struct Command {
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    pub id: u64,
}

#[derive(Debug, Deserialize)]
pub struct Response {
    pub id: u64,
    #[serde(default)]
    pub result: Value,
    pub error: Option<ResponseError>,
}

#[derive(Debug, Deserialize)]
pub struct ResponseError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct Event {
    pub method: String,
    pub params: Value,
}

#[derive(Debug)]
pub struct Session {
    pub target_id: String,
    pub session_id: String,
    pub timeout: Duration,
}

pub mod browser {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    #[derive(Debug, Serialize)]
    pub struct GetVersion;

    #[derive(Debug, Deserialize)]
    pub struct Version {
        #[serde(rename = "protocolVersion")]
        pub protocol_version: String,
        pub product: String,
        pub revision: String,
        #[serde(rename = "userAgent")]
        pub user_agent: String,
        #[serde(rename = "jsVersion")]
        pub js_version: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Bounds {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub left: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub top: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub width: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub height: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub window_state: Option<WindowState>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub enum WindowState {
        Normal,
        Minimized,
        Maximized,
        Fullscreen,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub enum BrowserCommandId {
        OpenTabSearch,
        CloseTabSearch,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Histogram {
        pub name: String,
        pub sum: i32,
        pub count: i32,
        pub buckets: Vec<Bucket>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Bucket {
        pub low: i32,
        pub high: i32,
        pub count: i32,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PermissionDescriptor {
        pub name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub sysex: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub user_visible_only: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub allow_without_sanitization: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub allow_without_gesture: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub pan_tilt_zoom: Option<bool>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum PermissionSetting {
        Granted,
        Denied,
        Prompt,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub enum PermissionType {
        Ar,
        AudioCapture,
        AutomaticFullscreen,
        BackgroundFetch,
        BackgroundSync,
        CameraPanTiltZoom,
        CapturedSurfaceControl,
        ClipboardReadWrite,
        ClipboardSanitizedWrite,
        DisplayCapture,
        DurableStorage,
        Geolocation,
        HandTracking,
        IdleDetection,
        KeyboardLock,
        LocalFonts,
        LocalNetworkAccess,
        Midi,
        MidiSysex,
        Nfc,
        Notifications,
        PaymentHandler,
        PeriodicBackgroundSync,
        PointerLock,
        ProtectedMediaIdentifier,
        Sensors,
        SmartCard,
        SpeakerSelection,
        StorageAccess,
        TopLevelStorageAccess,
        VideoCapture,
        Vr,
        WakeLockScreen,
        WakeLockSystem,
        WebAppInstallation,
        WebPrinting,
        WindowManagement,
    }

    // Command Messages
    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Close;

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct GetWindowBounds {
        pub window_id: i32,
    }

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct GetWindowForTarget {
        pub target_id: String,
    }

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SetWindowBounds {
        pub window_id: i32,
        pub bounds: Bounds,
    }

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct GetHistogram {
        pub name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub delta: Option<bool>,
    }

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct GetHistograms {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub query: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub delta: Option<bool>,
    }

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SetPermission {
        pub permission: PermissionDescriptor,
        pub setting: PermissionSetting,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub origin: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub browser_context_id: Option<String>,
    }

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct GrantPermissions {
        pub permissions: Vec<PermissionType>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub origin: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub browser_context_id: Option<String>,
    }

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ResetPermissions {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub browser_context_id: Option<String>,
    }

    // Response Messages
    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct GetWindowBoundsResponse {
        pub bounds: Bounds,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct GetWindowForTargetResponse {
        pub window_id: i32,
        pub bounds: Bounds,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct GetHistogramResponse {
        pub histogram: Histogram,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct GetHistogramsResponse {
        pub histograms: Vec<Histogram>,
    }
}

pub mod target {
    use serde::{Deserialize, Serialize};
    use serde_json::Value;

    #[derive(Debug, Serialize)]
    pub struct CreateTarget {
        pub url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub width: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub height: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub browser_context_id: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    pub struct CreateTargetResponse {
        #[serde(rename = "targetId")]
        pub target_id: String,
    }

    #[derive(Debug, Serialize)]
    pub struct AttachToTarget {
        #[serde(rename = "targetId")]
        pub target_id: String,
        pub flatten: bool,
    }

    #[derive(Debug, Deserialize)]
    pub struct AttachToTargetResponse {
        #[serde(rename = "sessionId")]
        pub session_id: String,
    }

    #[derive(Debug, Deserialize)]
    pub struct TargetInfo {
        #[serde(rename = "targetId")]
        pub target_id: String,
        #[serde(rename = "type")]
        pub target_type: String,
        pub title: String,
        pub url: String,
        pub attached: bool,
        #[serde(rename = "browserContextId")]
        pub browser_context_id: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    pub struct TargetCreatedEvent {
        #[serde(rename = "targetInfo")]
        pub target_info: TargetInfo,
    }

    #[derive(Debug, Deserialize)]
    pub struct TargetDestroyedEvent {
        #[serde(rename = "targetId")]
        pub target_id: String,
    }
}

pub mod page {
    use serde::{Deserialize, Serialize};
    use serde_json::Value;

    #[derive(Debug, Serialize)]
    pub struct Navigate {
        pub url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub referrer: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub transition_type: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    pub struct NavigateResponse {
        #[serde(rename = "frameId")]
        pub frame_id: String,
        #[serde(rename = "loaderId")]
        pub loader_id: String,
    }

    #[derive(Debug, Deserialize)]
    pub struct FrameNavigatedEvent {
        pub frame: Frame,
    }

    #[derive(Debug, Deserialize)]
    pub struct Frame {
        pub id: String,
        #[serde(rename = "parentId")]
        pub parent_id: Option<String>,
        pub url: String,
        #[serde(rename = "securityOrigin")]
        pub security_origin: String,
        #[serde(rename = "mimeType")]
        pub mime_type: String,
    }
} 