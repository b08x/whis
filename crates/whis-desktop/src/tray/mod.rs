//! System Tray Module
//!
//! Manages the system tray icon, menu, and interactions.
//! Platform-specific implementations for macOS and Linux.
//!
//! ## Architecture
//!
//! ```text
//! tray/
//! ├── icons.rs     - Icon constants & loading
//! ├── menu.rs      - Menu updates based on state
//! ├── events.rs    - Event handlers (menu, icon clicks)
//! ├── setup.rs     - Tray initialization
//! └── mod.rs       - Public API (this file)
//! ```

pub mod events;
pub mod icons;
pub mod menu;
pub mod setup;

// Re-export public API
pub use setup::setup_tray;

// Re-export tray ID for external use
pub const TRAY_ID: &str = "whis-tray";
