//! GNOME dconf Shortcut Reading
//!
//! Provides functionality to read portal shortcuts directly from GNOME's dconf database.
//! This allows detecting shortcuts even when portal binding fails or returns errors.

/// Read the actual portal shortcut from dconf (GNOME)
/// Returns the shortcut in format like "Ctrl+Alt+M" if found
#[cfg(target_os = "linux")]
pub fn read_portal_shortcut_from_dconf() -> Option<String> {
    // Run: dconf dump /org/gnome/settings-daemon/global-shortcuts/
    let output = std::process::Command::new("dconf")
        .args(["dump", "/org/gnome/settings-daemon/global-shortcuts/"])
        .output()
        .ok()?;

    let dump = String::from_utf8_lossy(&output.stdout);

    // Look for toggle-recording in any app section
    // Format: shortcuts=[('toggle-recording', {'shortcuts': <['<Control><Alt>m']>, ...})]
    for line in dump.lines() {
        if line.contains("toggle-recording") && line.contains("shortcuts") {
            // Parse the GVariant format: <['<Control><Alt>m']>
            if let Some(start) = line.find("<['")
                && let Some(end) = line[start..].find("']>")
            {
                let raw = &line[start + 3..start + end];
                // Convert <Control><Alt>m to Ctrl+Alt+M
                return Some(convert_gvariant_shortcut(raw));
            }
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
pub fn read_portal_shortcut_from_dconf() -> Option<String> {
    None
}

/// Convert GVariant shortcut format to human-readable format
/// e.g., "<Control><Alt>m" -> "Ctrl+Alt+M"
#[cfg(target_os = "linux")]
fn convert_gvariant_shortcut(raw: &str) -> String {
    let converted = raw
        .replace("<Control>", "Ctrl+")
        .replace("<Alt>", "Alt+")
        .replace("<Shift>", "Shift+")
        .replace("<Super>", "Super+");

    // Uppercase the final key and handle trailing +
    if let Some(last_plus) = converted.rfind('+') {
        let (modifiers, key) = converted.split_at(last_plus + 1);
        format!("{}{}", modifiers, key.to_uppercase())
    } else {
        converted.to_uppercase()
    }
}
