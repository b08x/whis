//! Model Download Synchronization
//!
//! Provides global locks to prevent concurrent downloads of the same model type.
//! This ensures only one Whisper and one Parakeet download can happen at a time.

use std::sync::{Mutex, OnceLock};

// Global locks for preventing concurrent model downloads
static WHISPER_DOWNLOAD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static PARAKEET_DOWNLOAD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub fn get_whisper_lock() -> &'static Mutex<()> {
    WHISPER_DOWNLOAD_LOCK.get_or_init(|| Mutex::new(()))
}

pub fn get_parakeet_lock() -> &'static Mutex<()> {
    PARAKEET_DOWNLOAD_LOCK.get_or_init(|| Mutex::new(()))
}
