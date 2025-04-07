use actix::Message;
use serde_json::Value;

use crate::error::ChromeError;
use crate::protocol::browser::{
    Bounds, BrowserCommandId, Histogram, PermissionDescriptor,
    PermissionSetting, PermissionType, Version,
};

// Browser Domain Messages
#[derive(Debug, Message)]
#[rtype(result = "Result<(), ChromeError>")]
pub struct Close;

#[derive(Debug, Message)]
#[rtype(result = "Result<Version, ChromeError>")]
pub struct GetVersion;

#[derive(Debug, Message)]
#[rtype(result = "Result<(), ChromeError>")]
pub struct ResetPermissions {
    pub browser_context_id: Option<String>,
}

#[derive(Debug, Message)]
#[rtype(result = "Result<Bounds, ChromeError>")]
pub struct GetWindowBounds {
    pub window_id: i32,
}

#[derive(Debug, Message)]
#[rtype(result = "Result<(i32, Bounds), ChromeError>")]
pub struct GetWindowForTarget {
    pub target_id: String,
}

#[derive(Debug, Message)]
#[rtype(result = "Result<(), ChromeError>")]
pub struct SetWindowBounds {
    pub window_id: i32,
    pub bounds: Bounds,
}

#[derive(Debug, Message)]
#[rtype(result = "Result<Histogram, ChromeError>")]
pub struct GetHistogram {
    pub name: String,
    pub delta: Option<bool>,
}

#[derive(Debug, Message)]
#[rtype(result = "Result<Vec<Histogram>, ChromeError>")]
pub struct GetHistograms {
    pub query: Option<String>,
    pub delta: Option<bool>,
}

#[derive(Debug, Message)]
#[rtype(result = "Result<(), ChromeError>")]
pub struct SetPermission {
    pub permission: PermissionDescriptor,
    pub setting: PermissionSetting,
    pub origin: Option<String>,
    pub browser_context_id: Option<String>,
}

#[derive(Debug, Message)]
#[rtype(result = "Result<(), ChromeError>")]
pub struct GrantPermissions {
    pub permissions: Vec<PermissionType>,
    pub origin: Option<String>,
    pub browser_context_id: Option<String>,
}

#[derive(Debug, Message)]
#[rtype(result = "Result<(), ChromeError>")]
pub struct ExecuteBrowserCommand {
    pub command_id: BrowserCommandId,
} 