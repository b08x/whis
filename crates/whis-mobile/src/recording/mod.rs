//! Recording and transcription business logic.
//!
//! This module contains the core recording and transcription functionality,
//! separated from the Tauri command handlers in `commands/recording.rs`.
//!
//! ## Modules
//!
//! - `config` - Load transcription configuration from Tauri store
//! - `pipeline` - Post-processing, clipboard, and event handling
//! - `provider` - Provider API key lookup and validation helpers

pub mod config;
pub mod pipeline;
pub mod provider;
