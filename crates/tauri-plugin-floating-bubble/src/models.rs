use serde::{Deserialize, Serialize};

/// Options for configuring the floating bubble.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BubbleOptions {
    /// Size of the bubble in dp (density-independent pixels).
    /// Default: 60
    #[serde(default = "default_size")]
    pub size: i32,

    /// Initial X position of the bubble.
    /// Default: 0 (left edge)
    #[serde(default)]
    pub start_x: i32,

    /// Initial Y position of the bubble.
    /// Default: 100
    #[serde(default = "default_start_y")]
    pub start_y: i32,
}

fn default_size() -> i32 {
    60
}

fn default_start_y() -> i32 {
    100
}

/// Response from visibility check.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisibilityResponse {
    pub visible: bool,
}

/// Response from permission check.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionResponse {
    pub granted: bool,
}
