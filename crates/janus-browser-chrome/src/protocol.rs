//! Basic structures for Chrome DevTools Protocol (CDP) commands and events.
//! Using serde_json::Value for params/results for simplicity in Phase 2.

use serde::{Deserialize, Serialize};
use serde_json::Value;

// --- Commands ---

// Example: Target.createTarget command parameters
#[derive(Serialize, Debug)]
pub struct CreateTargetParams {
    pub url: String,
    // Add other options like width, height, browserContextId etc. later
}

// Example: Target.attachToTarget command parameters
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AttachToTargetParams {
    pub target_id: String,
    pub flatten: Option<bool>, // Recommended for easier session management
}

// Example: Page.navigate command parameters
#[derive(Serialize, Debug)]
pub struct NavigateParams<'a> {
    pub url: &'a str,
    // Add referrer, transitionType etc. later
}

// Example: Runtime.evaluate command parameters
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateParams<'a> {
    pub expression: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_id: Option<i64>, // Execution context ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_by_value: Option<bool>, // Return primitive values directly
    #[serde(skip_serializing_if = "Option::is_none")]
    pub await_promise: Option<bool>, // If expression returns promise, await it
                                     // Add timeout etc. later
}

// Example: Target.setDiscoverTargets command parameters
#[derive(Serialize, Debug)]
pub struct SetDiscoverTargetsParams {
    pub discover: bool,
}

// Browser.resetPermissions command parameters
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ResetPermissionsParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_context_id: Option<String>,
}

// --- Results ---

// Example: Target.createTarget result
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateTargetResult {
    pub target_id: String,
}

// Example: Target.attachToTarget result
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AttachToTargetResult {
    pub session_id: String,
}

// Example: Runtime.evaluate result
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateResult {
    pub result: RemoteObject,
    #[serde(default)]
    pub exception_details: Option<ExceptionDetails>,
}

// --- Events ---

// Example: Target.targetCreated event parameters
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TargetCreatedParams {
    pub target_info: TargetInfo,
}

// Example: Target.targetInfoChanged event parameters
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TargetInfoChangedParams {
    pub target_info: TargetInfo,
}

// Example: Target.detachedFromTarget event parameters
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DetachedFromTargetParams {
    pub session_id: String,
    // target_id might also be present
}

// Example: Target.targetDestroyed event parameters
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TargetDestroyedParams {
    pub target_id: String,
}

// --- Common Nested Types ---

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TargetInfo {
    pub target_id: String,
    #[serde(rename = "type")]
    pub type_: String, // e.g., "page", "browser", "service_worker"
    pub title: String,
    pub url: String,
    pub attached: bool,
    #[serde(default)]
    pub browser_context_id: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RemoteObject {
    #[serde(rename = "type")]
    pub type_: String, // e.g., "string", "number", "object", "undefined"
    pub subtype: Option<String>,     // e.g., "null", "array", "error"
    pub description: Option<String>, // String representation
    #[serde(default)]
    pub value: Value, // Primitive value or preview if not object
                                     // object_id if it's an object handle (needed for interaction)
                                     // preview, custom_preview if object/function
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionDetails {
    pub exception_id: i64,
    pub text: String, // Short description, e.g., "Uncaught"
    pub line_number: i64,
    pub column_number: i64,
    pub script_id: Option<String>,
    pub url: Option<String>,
    pub exception: Option<RemoteObject>, // Detailed exception object
    pub execution_context_id: i64,
    // stack_trace might be here
}
