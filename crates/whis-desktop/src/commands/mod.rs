//! Tauri Command Handlers
//!
//! This module organizes all Tauri command handlers by domain.
//! Each sub-module contains related commands that are exposed to the frontend.
//!
//! ## Architecture
//!
//! ```text
//! commands/
//! ├── system.rs          - System utilities (audio devices, exit, toggle cmd)
//! ├── validation.rs      - API key validators
//! ├── recording.rs       - Recording status commands
//! ├── settings.rs        - Settings management & config readiness
//! ├── shortcuts.rs       - Shortcut configuration
//! ├── models/            - Model download management
//! │   ├── downloads.rs   - Download locks
//! │   ├── whisper.rs     - Whisper model commands
//! │   ├── parakeet.rs    - Parakeet model commands (feature-gated)
//! │   └── mod.rs         - Public API
//! ├── presets.rs         - Preset CRUD
//! ├── ollama.rs          - Ollama integration
//! └── mod.rs             - Public API (this file)
//! ```

pub mod models;
pub mod ollama;
pub mod presets;
pub mod recording;
pub mod settings;
pub mod shortcuts;
pub mod system;
pub mod validation;

// Re-export all commands for tauri::generate_handler!

// System commands
pub use system::*;

// Validation commands
pub use validation::*;

// Recording commands
pub use recording::*;

// Settings commands
pub use settings::*;

// Shortcut commands
pub use shortcuts::*;

// Model commands
pub use models::*;

// Preset commands
pub use presets::*;

// Ollama commands
pub use ollama::*;
