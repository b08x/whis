//! IPC Toggle Server
//!
//! Provides Unix socket-based IPC for external toggle commands.
//! Allows CLI invocations like `whis-desktop --toggle` to communicate with the running instance.

use std::env;
use tauri::AppHandle;

/// Send toggle command to running instance via Unix socket
#[cfg(unix)]
pub fn send_toggle_command() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    use std::os::unix::net::UnixStream;

    let socket_path = socket_path();

    match UnixStream::connect(&socket_path) {
        Ok(mut stream) => {
            stream.write_all(b"toggle")?;
            println!("Toggle command sent");
            Ok(())
        }
        Err(e) => {
            eprintln!("Could not connect to running instance: {e}");
            eprintln!("Is whis-desktop running?");
            Err(e.into())
        }
    }
}

#[cfg(not(unix))]
pub fn send_toggle_command() -> Result<(), Box<dyn std::error::Error>> {
    Err("Unix sockets not available on this platform".into())
}

/// Start listening for IPC commands
#[cfg(unix)]
pub fn start_ipc_listener(app_handle: AppHandle) {
    let socket_path = socket_path();

    // Remove old socket if exists
    let _ = std::fs::remove_file(&socket_path);

    std::thread::spawn(move || {
        use std::io::Read;
        use std::os::unix::net::UnixListener;

        let listener = match UnixListener::bind(&socket_path) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Failed to create IPC socket: {e}");
                return;
            }
        };

        println!("IPC listener started at {socket_path}");

        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let mut buf = [0u8; 64];
                    if let Ok(n) = stream.read(&mut buf) {
                        let cmd = String::from_utf8_lossy(&buf[..n]);
                        if cmd.trim() == "toggle" {
                            println!("IPC: toggle command received");
                            let handle = app_handle.clone();
                            // Dispatch to Tauri's async runtime - the IPC thread has no Tokio runtime
                            tauri::async_runtime::spawn(async move {
                                crate::recording::toggle_recording(handle);
                            });
                        }
                    }
                }
                Err(e) => eprintln!("IPC connection error: {e}"),
            }
        }
    });
}

#[cfg(not(unix))]
pub fn start_ipc_listener(_app_handle: AppHandle) {
    // No-op on non-Unix platforms
}

#[cfg(unix)]
fn socket_path() -> String {
    let runtime_dir = env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
    format!("{runtime_dir}/whis-desktop.sock")
}
