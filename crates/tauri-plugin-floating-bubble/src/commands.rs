use tauri::{command, AppHandle, Runtime};

use crate::models::*;
use crate::FloatingBubbleExt;
use crate::Result;

/// Show the floating bubble overlay.
#[command]
pub(crate) async fn show_bubble<R: Runtime>(
    app: AppHandle<R>,
    options: Option<BubbleOptions>,
) -> Result<()> {
    app.floating_bubble().show(options.unwrap_or_default())
}

/// Hide the floating bubble overlay.
#[command]
pub(crate) async fn hide_bubble<R: Runtime>(app: AppHandle<R>) -> Result<()> {
    app.floating_bubble().hide()
}

/// Check if the floating bubble is currently visible.
#[command]
pub(crate) async fn is_bubble_visible<R: Runtime>(app: AppHandle<R>) -> Result<VisibilityResponse> {
    app.floating_bubble().is_visible()
}

/// Request the overlay permission (SYSTEM_ALERT_WINDOW).
#[command]
pub(crate) async fn request_overlay_permission<R: Runtime>(
    app: AppHandle<R>,
) -> Result<PermissionResponse> {
    app.floating_bubble().request_permission()
}

/// Check if the overlay permission (SYSTEM_ALERT_WINDOW) is granted.
#[command]
pub(crate) async fn has_overlay_permission<R: Runtime>(
    app: AppHandle<R>,
) -> Result<PermissionResponse> {
    app.floating_bubble().has_permission()
}

/// Set the bubble's visual state.
#[command]
pub(crate) async fn set_bubble_state<R: Runtime>(app: AppHandle<R>, state: String) -> Result<()> {
    app.floating_bubble().set_state(state)
}
