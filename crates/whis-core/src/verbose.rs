//! Verbose logging support for debugging whis operations.
//!
//! Use `set_verbose(true)` to enable verbose output, then use `verbose!()` macro
//! to print debug information.

use std::sync::atomic::{AtomicBool, Ordering};

static VERBOSE: AtomicBool = AtomicBool::new(false);

/// Enable or disable verbose logging
pub fn set_verbose(enabled: bool) {
    VERBOSE.store(enabled, Ordering::SeqCst);
}

/// Check if verbose logging is enabled
pub fn is_verbose() -> bool {
    VERBOSE.load(Ordering::SeqCst)
}

/// Log a formatted message if verbose mode is enabled
#[macro_export]
macro_rules! verbose {
    ($($arg:tt)*) => {
        if $crate::verbose::is_verbose() {
            eprintln!("[verbose] {}", format!($($arg)*));
        }
    };
}
