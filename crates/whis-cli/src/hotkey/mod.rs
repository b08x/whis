//! Cross-platform hotkey support
//!
//! - Linux: Uses rdev for keyboard grab (supports X11 and Wayland)
//! - Windows/macOS: Uses global-hotkey crate (Tauri-maintained)

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(not(target_os = "linux"))]
mod non_linux;
#[cfg(not(target_os = "linux"))]
pub use non_linux::*;
