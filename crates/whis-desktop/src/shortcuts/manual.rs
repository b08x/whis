//! Manual Shortcut Setup Instructions
//!
//! Provides brief instructions for users when automatic shortcut methods
//! (Tauri plugin, Portal, RdevGrab) are unavailable.

use whis_core::Compositor;

/// Print concise setup instructions for the user
pub fn print_manual_setup_instructions(_compositor: &Compositor, _shortcut: &str) {
    println!();
    println!("Shortcut not configured. Two options:");
    println!("  - System: Configure compositor shortcut -> whis-desktop --toggle");
    println!("  - Direct: Enable direct keyboard access (see Settings)");
    println!();
}
