use serde::de::DeserializeOwned;
use tauri::{AppHandle, Runtime, plugin::PluginApi};

use crate::models::*;

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<FloatingBubble<R>> {
    Ok(FloatingBubble(app.clone()))
}

/// Access to the floating bubble APIs (desktop stub - not supported).
pub struct FloatingBubble<R: Runtime>(AppHandle<R>);

impl<R: Runtime> FloatingBubble<R> {
    pub fn show(&self, _options: BubbleOptions) -> crate::Result<()> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn hide(&self) -> crate::Result<()> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn is_visible(&self) -> crate::Result<VisibilityResponse> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn request_permission(&self) -> crate::Result<PermissionResponse> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn has_permission(&self) -> crate::Result<PermissionResponse> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn set_state(&self, _state: String) -> crate::Result<()> {
        Err(crate::Error::UnsupportedPlatform)
    }
}
