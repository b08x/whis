//! Manual Shortcut Setup Instructions
//!
//! Provides compositor-specific instructions for users to manually configure
//! global shortcuts when automatic methods (Tauri plugin, Portal) are unavailable.

/// Print manual setup instructions for the user based on their compositor
pub fn print_manual_setup_instructions(compositor: &str, shortcut: &str) {
    println!();
    println!("=== Global Shortcuts Not Available ===");
    println!("Compositor: {compositor}");
    println!();
    println!("To use a keyboard shortcut, configure your compositor:");
    println!();
    match compositor.to_lowercase().as_str() {
        s if s.contains("gnome") => {
            println!("GNOME: Settings → Keyboard → Custom Shortcuts");
            println!("  Name: Whis Toggle Recording");
            println!("  Command: whis-desktop --toggle");
            println!("  Shortcut: {shortcut}");
        }
        s if s.contains("kde") || s.contains("plasma") => {
            println!("KDE: System Settings → Shortcuts → Custom Shortcuts");
            println!("  Command: whis-desktop --toggle");
        }
        s if s.contains("sway") => {
            println!("Sway: Add to ~/.config/sway/config:");
            println!(
                "  bindsym {} exec whis-desktop --toggle",
                shortcut.to_lowercase()
            );
        }
        s if s.contains("hyprland") => {
            println!("Hyprland: Add to ~/.config/hypr/hyprland.conf:");
            println!(
                "  bind = {}, exec, whis-desktop --toggle",
                shortcut.replace("+", ", ")
            );
        }
        _ => {
            println!("Configure your compositor to run: whis-desktop --toggle");
        }
    }
    println!();
}
