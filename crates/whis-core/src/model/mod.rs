//! Model Management Module
//!
//! This module provides unified download and verification utilities for
//! transcription models (Whisper, Parakeet).
//!
//! # Architecture
//!
//! ```text
//! ModelType Trait
//!   ├── WhisperModel   - Whisper.cpp GGML models
//!   └── ParakeetModel  - Parakeet ONNX models
//!
//! Generic Operations
//!   ├── download()     - Download with progress
//!   ├── ensure()       - Download if missing
//!   └── verify()       - Check if valid
//! ```
//!
//! # Usage
//!
//! ```rust
//! use whis_core::model::{self, whisper, parakeet};
//!
//! // Download Whisper model
//! let whisper = whisper::WhisperModel;
//! model::download::ensure(&whisper, "small")?;
//!
//! // Download Parakeet model
//! let parakeet = parakeet::ParakeetModel;
//! model::download::ensure(&parakeet, "parakeet-v3")?;
//! ```

pub mod download;
pub mod parakeet;
pub mod types;
pub mod whisper;

// Re-export commonly used types
pub use types::{ModelInfo, ModelType};
pub use whisper::WhisperModel;

#[cfg(feature = "local-transcription")]
pub use parakeet::ParakeetModel;

// Re-export default model names for convenience
pub const DEFAULT_MODEL: &str = whisper::DEFAULT_MODEL;

#[cfg(feature = "local-transcription")]
pub const DEFAULT_PARAKEET_MODEL: &str = parakeet::DEFAULT_MODEL;
