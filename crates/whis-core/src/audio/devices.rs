//! Audio device enumeration and management.

use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait};

use super::types::AudioDeviceInfo;

#[cfg(target_os = "linux")]
mod alsa_suppress {
    use std::os::raw::{c_char, c_int};
    use std::sync::Once;

    // Use a non-variadic function pointer type for the handler.
    // ALSA's actual signature is variadic, but since our handler ignores all args,
    // we can use a simpler signature that's compatible at the ABI level.
    type SndLibErrorHandlerT =
        unsafe extern "C" fn(*const c_char, c_int, *const c_char, c_int, *const c_char);

    #[link(name = "asound")]
    unsafe extern "C" {
        fn snd_lib_error_set_handler(handler: Option<SndLibErrorHandlerT>) -> c_int;
    }

    // No-op error handler - does nothing, suppresses all ALSA errors
    unsafe extern "C" fn silent_error_handler(
        _file: *const c_char,
        _line: c_int,
        _function: *const c_char,
        _err: c_int,
        _fmt: *const c_char,
    ) {
        // Intentionally empty - suppress all ALSA error output
    }

    static INIT: Once = Once::new();

    /// Initialize ALSA error suppression.
    ///
    /// NOTE: This function can be safely removed without affecting functionality.
    /// It only suppresses noisy log output about unavailable PCM plugins (pulse, jack, oss).
    /// The unsafe FFI code here is purely cosmetic - audio works fine without it.
    pub fn init() {
        INIT.call_once(|| {
            // SAFETY: We provide a valid no-op error handler function.
            // This suppresses ALSA's error messages about unavailable PCM plugins.
            unsafe {
                snd_lib_error_set_handler(Some(silent_error_handler));
            }
        });
    }
}

#[cfg(not(target_os = "linux"))]
mod alsa_suppress {
    pub fn init() {}
}

/// List all available audio input devices on the system.
///
/// # Returns
/// A vector of audio device information, including device names and default status.
///
/// # Errors
/// Returns an error if no audio input devices are found.
pub fn list_audio_devices() -> Result<Vec<AudioDeviceInfo>> {
    alsa_suppress::init();

    let host = cpal::default_host();
    let default_device_name = host
        .default_input_device()
        .and_then(|d| d.description().ok())
        .map(|d| d.to_string());

    let mut devices = Vec::new();
    for device in host.input_devices()? {
        if let Ok(desc) = device.description() {
            let name = desc.to_string();
            devices.push(AudioDeviceInfo {
                name: name.clone(),
                is_default: default_device_name.as_ref() == Some(&name),
            });
        }
    }

    if devices.is_empty() {
        anyhow::bail!("No audio input devices found");
    }

    Ok(devices)
}

/// Initialize platform-specific audio system.
///
/// On Linux, this suppresses ALSA error messages about unavailable PCM plugins.
/// On other platforms, this is a no-op.
pub(super) fn init_platform() {
    alsa_suppress::init();
}
