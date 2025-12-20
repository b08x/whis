use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
#[cfg(feature = "ffmpeg")]
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Suppress ALSA warnings on Linux about unavailable PCM plugins (pulse, jack, oss).
///
/// NOTE: This module can be safely removed without affecting functionality.
/// It only suppresses noisy log output during development (e.g., "Unknown PCM pulse").
/// The unsafe FFI code here is purely cosmetic - audio works fine without it.
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

/// Threshold for chunking (files larger than this get split)
const CHUNK_THRESHOLD_BYTES: usize = 20 * 1024 * 1024; // 20 MB
/// Duration of each chunk in seconds
const CHUNK_DURATION_SECS: usize = 300; // 5 minutes
/// Overlap between chunks in seconds (to avoid cutting words)
const CHUNK_OVERLAP_SECS: usize = 2;

/// A chunk of audio data ready for transcription
#[derive(Clone)]
pub struct AudioChunk {
    /// MP3 audio data
    pub data: Vec<u8>,
    /// Chunk index (0-based, for ordering)
    pub index: usize,
    /// Whether this chunk has overlap from the previous chunk
    pub has_leading_overlap: bool,
}

/// Output of a completed recording - either a single file or multiple chunks
pub enum RecordingOutput {
    /// Small file that can be transcribed directly
    Single(Vec<u8>),
    /// Large file split into chunks for parallel transcription
    Chunked(Vec<AudioChunk>),
}

/// Recording data extracted from AudioRecorder after stopping.
/// This struct is Send-safe (unlike AudioRecorder on macOS where cpal::Stream isn't Send).
pub struct RecordingData {
    samples: Vec<f32>,
    sample_rate: u32,
    channels: u16,
}

pub struct AudioRecorder {
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
    stream: Option<cpal::Stream>,
}

// SAFETY: AudioRecorder is always used behind a Mutex in AppState, ensuring
// single-threaded access. The cpal::Stream on macOS contains non-Send types
// (AudioObjectPropertyListener with FnMut), but we never move the AudioRecorder
// between threads - it's created, used, and dropped on the same thread.
// The Mutex only protects against concurrent access within the Tauri async runtime.
//
// TODO(macos): Refactor to eliminate this unsafe impl. Possible approaches:
// 1. Channel-based: Move Stream to dedicated audio thread, communicate via mpsc channels
// 2. Actor pattern: AudioRecorder sends commands to an actor that owns the Stream
// Requires macOS hardware for testing.
#[cfg(target_os = "macos")]
unsafe impl Send for AudioRecorder {}

impl AudioRecorder {
    pub fn new() -> Result<Self> {
        Ok(AudioRecorder {
            samples: Arc::new(Mutex::new(Vec::new())),
            sample_rate: 44100, // Default sample rate
            channels: 1,        // Default channels
            stream: None,
        })
    }

    /// Start recording with an optional specific device name
    /// If device_name is None, uses the system default input device
    pub fn start_recording(&mut self) -> Result<()> {
        self.start_recording_with_device(None)
    }

    /// Start recording with a specific device name
    pub fn start_recording_with_device(&mut self, device_name: Option<&str>) -> Result<()> {
        alsa_suppress::init();
        let host = cpal::default_host();

        let device = if let Some(name) = device_name {
            // Try to find device by name
            host.input_devices()?
                .find(|d| d.name().map(|n| n == name).unwrap_or(false))
                .with_context(|| format!("Audio device '{}' not found", name))?
        } else {
            // Use default device
            host.default_input_device()
                .context("No input device available")?
        };

        let actual_device_name = device.name().unwrap_or_else(|_| "<unknown>".to_string());
        crate::verbose!("Audio device: {}", actual_device_name);

        let config = device
            .default_input_config()
            .context("Failed to get default input config")?;

        // Force mono on Android - emulators and some devices don't support stereo input
        #[cfg(target_os = "android")]
        let channels = 1u16;
        #[cfg(not(target_os = "android"))]
        let channels = config.channels();

        self.sample_rate = config.sample_rate().0;
        self.channels = channels;

        crate::verbose!(
            "Audio config: {} Hz, {} channel(s)",
            self.sample_rate,
            self.channels
        );

        let stream_config = cpal::StreamConfig {
            channels,
            sample_rate: config.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        };

        let samples = self.samples.clone();
        samples.lock().unwrap().clear();

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                self.build_stream::<f32>(&device, &stream_config, samples)?
            }
            cpal::SampleFormat::I16 => {
                self.build_stream::<i16>(&device, &stream_config, samples)?
            }
            cpal::SampleFormat::U16 => {
                self.build_stream::<u16>(&device, &stream_config, samples)?
            }
            _ => anyhow::bail!("Unsupported sample format"),
        };

        stream.play()?;

        // Store stream to keep it alive; dropping it will release the microphone
        self.stream = Some(stream);

        Ok(())
    }

    fn build_stream<T>(
        &self,
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        samples: Arc<Mutex<Vec<f32>>>,
    ) -> Result<cpal::Stream>
    where
        T: cpal::Sample + cpal::SizedSample,
        f32: cpal::FromSample<T>,
    {
        let err_fn = |err| eprintln!("Error in audio stream: {err}");

        let stream = device.build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                let mut samples = samples.lock().unwrap();
                for &sample in data {
                    samples.push(cpal::Sample::from_sample(sample));
                }
            },
            err_fn,
            None,
        )?;

        Ok(stream)
    }

    /// Stop recording and return the recording data.
    /// The stream is dropped here, making the returned RecordingData Send-safe.
    pub fn stop_recording(&mut self) -> Result<RecordingData> {
        // Drop the stream first to release the microphone
        self.stream = None;

        // Take ownership of samples and clear the buffer
        let samples: Vec<f32> = {
            let mut guard = self.samples.lock().unwrap();
            std::mem::take(&mut *guard)
        };

        if samples.is_empty() {
            crate::verbose!("No audio samples captured");
            anyhow::bail!("No audio data recorded");
        }

        let duration_secs = samples.len() as f32 / self.sample_rate as f32 / self.channels as f32;
        crate::verbose!("Recorded {} samples ({:.1}s)", samples.len(), duration_secs);

        Ok(RecordingData {
            samples,
            sample_rate: self.sample_rate,
            channels: self.channels,
        })
    }

    /// Stop recording and finalize in one step (convenience method for single-threaded use).
    pub fn finalize_recording(&mut self) -> Result<RecordingOutput> {
        self.stop_recording()?.finalize()
    }
}

impl RecordingData {
    /// Finalize the recording by converting samples to MP3.
    /// This is Send-safe and can be called from spawn_blocking.
    pub fn finalize(self) -> Result<RecordingOutput> {
        // Try to convert the entire recording first
        let mp3_data = self.samples_to_mp3(&self.samples, "main")?;

        // If at or under threshold, return as single file (fast path)
        if mp3_data.len() <= CHUNK_THRESHOLD_BYTES {
            return Ok(RecordingOutput::Single(mp3_data));
        }

        // File is too large - need to chunk it
        let samples_per_second = self.sample_rate as usize * self.channels as usize;
        let chunk_samples = CHUNK_DURATION_SECS * samples_per_second;
        let overlap_samples = CHUNK_OVERLAP_SECS * samples_per_second;

        let mut chunks = Vec::new();
        let mut chunk_start = 0usize;
        let mut chunk_index = 0usize;

        while chunk_start < self.samples.len() {
            let chunk_end = (chunk_start + chunk_samples).min(self.samples.len());
            let chunk_slice = &self.samples[chunk_start..chunk_end];

            // Convert this chunk to MP3
            let chunk_mp3 = self.samples_to_mp3(chunk_slice, &format!("chunk{chunk_index}"))?;

            chunks.push(AudioChunk {
                data: chunk_mp3,
                index: chunk_index,
                has_leading_overlap: chunk_index > 0,
            });

            chunk_index += 1;

            // Check if we've reached the end
            if chunk_end >= self.samples.len() {
                break;
            }

            // Move to next chunk, stepping back by overlap amount
            chunk_start = chunk_end.saturating_sub(overlap_samples);
        }

        Ok(RecordingOutput::Chunked(chunks))
    }

    /// Convert raw f32 samples to MP3 data using FFmpeg (desktop) or embedded encoder (mobile)
    #[cfg(feature = "ffmpeg")]
    fn samples_to_mp3(&self, samples: &[f32], suffix: &str) -> Result<Vec<u8>> {
        self.samples_to_mp3_ffmpeg(samples, suffix)
    }

    #[cfg(all(feature = "embedded-encoder", not(feature = "ffmpeg")))]
    fn samples_to_mp3(&self, samples: &[f32], _suffix: &str) -> Result<Vec<u8>> {
        self.samples_to_mp3_embedded(samples)
    }

    #[cfg(not(any(feature = "ffmpeg", feature = "embedded-encoder")))]
    fn samples_to_mp3(&self, _samples: &[f32], _suffix: &str) -> Result<Vec<u8>> {
        anyhow::bail!(
            "No MP3 encoder available. Enable either 'ffmpeg' or 'embedded-encoder' feature."
        )
    }

    /// Convert samples to MP3 using FFmpeg process (desktop)
    #[cfg(feature = "ffmpeg")]
    fn samples_to_mp3_ffmpeg(&self, samples: &[f32], suffix: &str) -> Result<Vec<u8>> {
        // Convert f32 samples to i16 for WAV format
        let i16_samples: Vec<i16> = samples
            .iter()
            .map(|&s| {
                let clamped = s.clamp(-1.0, 1.0);
                (clamped * i16::MAX as f32) as i16
            })
            .collect();

        // Use unique temp file names to support parallel FFmpeg calls
        let temp_dir = std::env::temp_dir();
        let unique_id = format!(
            "{}_{}_{suffix}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        );
        let wav_path = temp_dir.join(format!("whis_{unique_id}.wav"));
        let mp3_path = temp_dir.join(format!("whis_{unique_id}.mp3"));

        {
            let spec = hound::WavSpec {
                channels: self.channels,
                sample_rate: self.sample_rate,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };

            let mut writer = hound::WavWriter::create(&wav_path, spec)?;
            for sample in i16_samples {
                writer.write_sample(sample)?;
            }
            writer.finalize()?;
        }

        // Convert WAV to MP3 using FFmpeg
        let output = std::process::Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-i",
                wav_path.to_str().unwrap(),
                "-codec:a",
                "libmp3lame",
                "-b:a",
                "128k",
                "-y",
                mp3_path.to_str().unwrap(),
            ])
            .output()
            .context("Failed to execute ffmpeg. Make sure ffmpeg is installed.")?;

        // Clean up the temporary WAV file
        let _ = std::fs::remove_file(&wav_path);

        if !output.status.success() {
            let _ = std::fs::remove_file(&mp3_path);
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("FFmpeg conversion failed: {stderr}");
        }

        // Read the MP3 file
        let mp3_data = std::fs::read(&mp3_path).context("Failed to read converted MP3 file")?;

        // Clean up the temporary MP3 file
        let _ = std::fs::remove_file(&mp3_path);

        Ok(mp3_data)
    }

    /// Convert samples to MP3 using embedded LAME encoder (mobile)
    #[cfg(feature = "embedded-encoder")]
    #[allow(dead_code)] // Used only when ffmpeg feature is disabled
    fn samples_to_mp3_embedded(&self, samples: &[f32]) -> Result<Vec<u8>> {
        use mp3lame_encoder::{Builder, FlushNoGap, InterleavedPcm, MonoPcm};

        // Convert f32 samples to i16
        let i16_samples: Vec<i16> = samples
            .iter()
            .map(|&s| {
                let clamped = s.clamp(-1.0, 1.0);
                (clamped * i16::MAX as f32) as i16
            })
            .collect();

        // Build the encoder
        let mut builder = Builder::new().context("Failed to create LAME builder")?;
        builder
            .set_num_channels(self.channels as u8)
            .map_err(|e| anyhow::anyhow!("Failed to set channels: {:?}", e))?;
        builder
            .set_sample_rate(self.sample_rate)
            .map_err(|e| anyhow::anyhow!("Failed to set sample rate: {:?}", e))?;
        builder
            .set_brate(mp3lame_encoder::Bitrate::Kbps128)
            .map_err(|e| anyhow::anyhow!("Failed to set bitrate: {:?}", e))?;
        builder
            .set_quality(mp3lame_encoder::Quality::Best)
            .map_err(|e| anyhow::anyhow!("Failed to set quality: {:?}", e))?;

        let mut encoder = builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to initialize LAME encoder: {:?}", e))?;

        // Prepare output buffer
        let mut mp3_data = Vec::new();
        let max_size = mp3lame_encoder::max_required_buffer_size(i16_samples.len());
        mp3_data.reserve(max_size);

        // Encode based on channel count
        let encoded_size = if self.channels == 1 {
            let input = MonoPcm(&i16_samples);
            encoder
                .encode(input, mp3_data.spare_capacity_mut())
                .map_err(|e| anyhow::anyhow!("Failed to encode MP3: {:?}", e))?
        } else {
            let input = InterleavedPcm(&i16_samples);
            encoder
                .encode(input, mp3_data.spare_capacity_mut())
                .map_err(|e| anyhow::anyhow!("Failed to encode MP3: {:?}", e))?
        };

        // SAFETY: encoder.encode returns the number of bytes written to the buffer.
        // The mp3lame-encoder API requires MaybeUninit<u8> output and guarantees
        // that exactly encoded_size bytes are initialized on success.
        unsafe {
            mp3_data.set_len(encoded_size);
        }

        // Flush remaining data
        let flush_size = encoder
            .flush::<FlushNoGap>(mp3_data.spare_capacity_mut())
            .map_err(|e| anyhow::anyhow!("Failed to flush MP3 encoder: {:?}", e))?;

        // SAFETY: flush returns the number of additional bytes written.
        // The encoder guarantees flush_size bytes are initialized.
        unsafe {
            mp3_data.set_len(mp3_data.len() + flush_size);
        }

        Ok(mp3_data)
    }
}

/// Load audio from a file, converting to MP3 if needed
#[cfg(feature = "ffmpeg")]
pub fn load_audio_file(path: &Path) -> Result<RecordingOutput> {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mp3_data = match extension.as_str() {
        "mp3" => {
            // Read MP3 directly
            std::fs::read(path).context("Failed to read MP3 file")?
        }
        "wav" | "m4a" | "ogg" | "flac" | "webm" | "aac" | "opus" => {
            // Convert to MP3 using FFmpeg
            convert_file_to_mp3(path)?
        }
        _ => {
            anyhow::bail!(
                "Unsupported audio format: '{}'. Supported: mp3, wav, m4a, ogg, flac, webm, aac, opus",
                extension
            );
        }
    };

    classify_recording_output(mp3_data)
}

#[cfg(not(feature = "ffmpeg"))]
pub fn load_audio_file(_path: &Path) -> Result<RecordingOutput> {
    anyhow::bail!("File input requires the 'ffmpeg' feature (not available in mobile builds)")
}

/// Load audio from stdin
#[cfg(feature = "ffmpeg")]
pub fn load_audio_stdin(format: &str) -> Result<RecordingOutput> {
    let mut data = Vec::new();
    std::io::stdin()
        .read_to_end(&mut data)
        .context("Failed to read audio from stdin")?;

    if data.is_empty() {
        anyhow::bail!("No audio data received from stdin");
    }

    let mp3_data = match format.to_lowercase().as_str() {
        "mp3" => data, // Already MP3
        "wav" | "m4a" | "ogg" | "flac" | "webm" | "aac" | "opus" => {
            // Convert stdin data to MP3 using FFmpeg
            convert_stdin_to_mp3(&data, format)?
        }
        _ => {
            anyhow::bail!(
                "Unsupported stdin format: '{}'. Supported: mp3, wav, m4a, ogg, flac, webm, aac, opus",
                format
            );
        }
    };

    classify_recording_output(mp3_data)
}

#[cfg(not(feature = "ffmpeg"))]
pub fn load_audio_stdin(_format: &str) -> Result<RecordingOutput> {
    anyhow::bail!("Stdin input requires the 'ffmpeg' feature (not available in mobile builds)")
}

/// Classify MP3 data into Single or Chunked based on size
#[cfg(feature = "ffmpeg")]
fn classify_recording_output(mp3_data: Vec<u8>) -> Result<RecordingOutput> {
    if mp3_data.len() <= CHUNK_THRESHOLD_BYTES {
        Ok(RecordingOutput::Single(mp3_data))
    } else {
        // For pre-encoded MP3 files, we can't easily split by time
        // For now, just use as single file - chunking is mainly for recordings
        // where we have raw samples and can calculate exact time boundaries
        crate::verbose!(
            "Large file ({:.1} MB) - processing as single file",
            mp3_data.len() as f64 / 1024.0 / 1024.0
        );
        Ok(RecordingOutput::Single(mp3_data))
    }
}

/// Convert an audio file to MP3 using FFmpeg
#[cfg(feature = "ffmpeg")]
fn convert_file_to_mp3(input_path: &Path) -> Result<Vec<u8>> {
    let temp_dir = std::env::temp_dir();
    let unique_id = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    );
    let mp3_path = temp_dir.join(format!("whis_convert_{unique_id}.mp3"));

    crate::verbose!("Converting {} to MP3...", input_path.display());

    let output = std::process::Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-i",
            input_path.to_str().unwrap(),
            "-codec:a",
            "libmp3lame",
            "-b:a",
            "128k",
            "-y",
            mp3_path.to_str().unwrap(),
        ])
        .output()
        .context("Failed to execute ffmpeg. Make sure ffmpeg is installed.")?;

    if !output.status.success() {
        let _ = std::fs::remove_file(&mp3_path);
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("FFmpeg conversion failed: {stderr}");
    }

    let mp3_data = std::fs::read(&mp3_path).context("Failed to read converted MP3 file")?;
    let _ = std::fs::remove_file(&mp3_path);

    crate::verbose!("Converted to {:.1} KB MP3", mp3_data.len() as f64 / 1024.0);

    Ok(mp3_data)
}

/// Convert stdin audio data to MP3 using FFmpeg
#[cfg(feature = "ffmpeg")]
fn convert_stdin_to_mp3(data: &[u8], format: &str) -> Result<Vec<u8>> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let temp_dir = std::env::temp_dir();
    let unique_id = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    );
    let mp3_path = temp_dir.join(format!("whis_stdin_{unique_id}.mp3"));

    crate::verbose!("Converting stdin ({} format) to MP3...", format);

    // Use FFmpeg with pipe input
    let mut child = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            format,
            "-i",
            "pipe:0", // Read from stdin
            "-codec:a",
            "libmp3lame",
            "-b:a",
            "128k",
            "-y",
            mp3_path.to_str().unwrap(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn ffmpeg process")?;

    // Write input data to FFmpeg's stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(data)
            .context("Failed to write audio data to ffmpeg")?;
    }

    let output = child.wait_with_output()?;

    if !output.status.success() {
        let _ = std::fs::remove_file(&mp3_path);
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("FFmpeg stdin conversion failed: {stderr}");
    }

    let mp3_data = std::fs::read(&mp3_path).context("Failed to read converted MP3 file")?;
    let _ = std::fs::remove_file(&mp3_path);

    crate::verbose!("Converted to {:.1} KB MP3", mp3_data.len() as f64 / 1024.0);

    Ok(mp3_data)
}

/// Audio device information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub is_default: bool,
}

/// List available audio input devices
pub fn list_audio_devices() -> Result<Vec<AudioDeviceInfo>> {
    alsa_suppress::init();
    let host = cpal::default_host();
    let default_device_name = host.default_input_device().and_then(|d| d.name().ok());

    let mut devices = Vec::new();
    for device in host.input_devices()? {
        if let Ok(name) = device.name() {
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
