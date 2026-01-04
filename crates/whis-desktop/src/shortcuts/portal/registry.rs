//! XDG Portal Registry Integration
//!
//! Handles app_id registration with the xdg-desktop-portal Registry interface.
//! This is required for native (non-Flatpak) apps to use portal features like GlobalShortcuts.
//! Without registration, the portal uses cgroup-based detection which fails when running from terminal.

/// Register app_id with the xdg-desktop-portal Registry
/// This is required for native (non-Flatpak) apps to use portal features like GlobalShortcuts.
/// Without this, the portal uses cgroup-based detection which fails when running from terminal.
#[cfg(target_os = "linux")]
pub async fn register_app_with_portal() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use std::collections::HashMap;
    use zbus::Connection;

    println!("Registering app_id 'ink.whis.Whis' with portal...");

    let connection = Connection::session().await?;

    // Call org.freedesktop.host.portal.Registry.Register
    let result: Result<(), zbus::Error> = connection
        .call_method(
            Some("org.freedesktop.portal.Desktop"),
            "/org/freedesktop/portal/desktop",
            Some("org.freedesktop.host.portal.Registry"),
            "Register",
            &(
                "ink.whis.Whis",
                HashMap::<String, zbus::zvariant::Value>::new(),
            ),
        )
        .await
        .map(|_: zbus::Message| ());

    match result {
        Ok(_) => {
            println!("Successfully registered app_id with portal");
            Ok(())
        }
        Err(e) => {
            // Registry might not be available (older portals), continue anyway
            println!("Portal Registry registration failed (may be unavailable): {e}");
            // Don't return error - this is optional for newer portals
            Ok(())
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub async fn register_app_with_portal() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Ok(())
}
