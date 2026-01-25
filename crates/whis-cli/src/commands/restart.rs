use crate::ipc;
use anyhow::Result;

pub fn run(preset_name: Option<String>) -> Result<()> {
    // Stop the service if running
    if ipc::is_service_running() {
        let mut client = ipc::IpcClient::connect()?;
        let _ = client.send_message(ipc::IpcMessage::Stop)?;
        println!("Service stopped");

        // Wait a moment for graceful shutdown
        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    // Start the service with optional preset
    crate::commands::start::run(false, preset_name)
}
