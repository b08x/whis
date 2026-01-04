//! Model Management Commands
//!
//! Provides commands for downloading and managing transcription models.
//!
//! ## Architecture
//!
//! ```text
//! models/
//! ├── downloads.rs   - Download lock synchronization
//! ├── whisper.rs     - Whisper model management
//! ├── parakeet.rs    - Parakeet model management
//! └── mod.rs         - Public API
//! ```

pub mod downloads;
pub mod whisper;

#[cfg(feature = "local-transcription")]
pub mod parakeet;

// Re-export all whisper types and commands
pub use whisper::*;

// Re-export all parakeet types and commands (conditionally)
#[cfg(feature = "local-transcription")]
pub use parakeet::*;
