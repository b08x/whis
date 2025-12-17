# Chapter 14b: Local Transcription

Cloud transcription is convenient, but some users need privacy, offline capability, or simply want to avoid API costs. This chapter explores how Whis implements fully local transcription using embedded Whisper models and Ollama for polishing.

## Why Local Transcription?

| Concern | Cloud | Local |
|---------|-------|-------|
| Privacy | Audio sent to third parties | Audio never leaves your machine |
| Cost | Per-minute pricing | Free after setup |
| Offline | Requires internet | Works anywhere |
| Latency | Network round-trip | Direct processing |
| Quality | State-of-the-art models | Good, but smaller models |

Local transcription gives you control. Your voice data stays on your hardware.

## Two Approaches

Whis offers two paths to local transcription:

```
                        Your Machine
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│   ┌─────────────┐      ┌─────────────────────────────────────┐  │
│   │   whis CLI  │      │         Transcription Options       │  │
│   │             │      │                                     │  │
│   │  Record     │─────▶│  A) local-whisper (embedded)        │  │
│   │  Audio      │      │     whisper.cpp compiled into binary│  │
│   │             │      │                                     │  │
│   │             │      │  B) remote-whisper (server)         │  │
│   │             │─────▶│     faster-whisper-server in Docker │  │
│   └─────────────┘      └─────────────────────────────────────┘  │
│          │                                                      │
│          │ (optional polish)                                    │
│          ▼                                                      │
│   ┌─────────────┐                                               │
│   │   Ollama    │  Local LLM for cleaning up transcripts        │
│   │  :11434     │  (grammar, filler words, corrections)         │
│   └─────────────┘                                               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Option A: Embedded (`local-whisper`)

Whisper.cpp is compiled directly into the whis binary via the `whisper-rs` crate.

**Pros**: Single binary, no server to manage
**Cons**: Larger binary (~50MB+), CPU-only (no GPU acceleration)

### Option B: Server (`remote-whisper`)

A whisper server runs in Docker, whis connects via HTTP.

**Pros**: GPU acceleration, shared across machines, smaller client binary
**Cons**: Docker dependency, server management overhead

## The `local-whisper` Feature Flag

Local whisper is enabled by default but can be disabled to reduce binary size:

```toml
# Cargo.toml
[features]
default = ["ffmpeg", "clipboard", "local-whisper"]
local-whisper = ["whisper-rs", "rubato", "minimp3"]
```

**Dependencies added**:
- `whisper-rs` (v0.15) - Rust bindings for whisper.cpp
- `rubato` (v0.15) - FFT-based audio resampling
- `minimp3` (v0.5) - Pure Rust MP3 decoder

**Binary size impact**:
- Without `local-whisper`: ~5MB
- With `local-whisper`: ~50MB+

## The Model Management Module

Before transcription, users need a Whisper model file. The `model.rs` module handles downloading and managing these models.

### Available Models

**From `whis-core/src/model.rs:11-32`**:

```rust
pub const WHISPER_MODELS: &[(&str, &str, &str)] = &[
    (
        "tiny",
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
        "~75 MB - Fastest, lower quality",
    ),
    (
        "base",
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
        "~142 MB - Fast, decent quality",
    ),
    (
        "small",
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
        "~466 MB - Balanced (recommended)",
    ),
    (
        "medium",
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
        "~1.5 GB - Better quality, slower",
    ),
];

pub const DEFAULT_MODEL: &str = "small";
```

Each tuple contains: `(name, download_url, description)`.

### Model Storage Location

**From `whis-core/src/model.rs:38-43`**:

```rust
pub fn default_models_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("whis")
        .join("models")
}
```

**Platform paths**:
- **Linux**: `~/.local/share/whis/models/`
- **macOS**: `~/Library/Application Support/whis/models/`
- **Windows**: `C:\Users\<name>\AppData\Local\whis\models\`

### Downloading Models

**From `whis-core/src/model.rs:64-139`**:

```rust
pub fn download_model(model_name: &str, dest: &Path) -> Result<()> {
    let url = get_model_url(model_name).ok_or_else(|| {
        anyhow!("Unknown model: {}. Available: tiny, base, small, medium", model_name)
    })?;

    // Create parent directory if needed
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).context("Failed to create models directory")?;
    }

    eprintln!("Downloading whisper model '{}'...", model_name);

    // Download with 10-minute timeout for large files
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()?;

    let mut response = client.get(url).send()?;
    let total_size = response.content_length();

    // Write to temp file, then rename on success
    let temp_path = dest.with_extension("bin.tmp");
    let mut file = fs::File::create(&temp_path)?;

    // ... progress display loop ...

    fs::rename(&temp_path, dest)?;
    Ok(())
}
```

**Key design decisions**:

1. **Temp file + rename**: Prevents corrupt partial downloads from appearing as valid models
2. **10-minute timeout**: Large models (1.5GB for medium) need time to download
3. **Progress display**: Users see download percentage to stderr

### Ensuring a Model Exists

**From `whis-core/src/model.rs:142-151`**:

```rust
pub fn ensure_model(model_name: &str) -> Result<PathBuf> {
    let path = default_model_path(model_name);

    if model_exists(&path) {
        return Ok(path);
    }

    download_model(model_name, &path)?;
    Ok(path)
}
```

This is the primary API: "give me this model, downloading if necessary."

## Audio Resampling

Whisper.cpp requires audio in a specific format: **16kHz mono float32 PCM**. The `resample.rs` module handles conversion.

### Why Resampling?

Whis records audio as MP3 (typically 44.1kHz stereo). Whisper needs:
- **16kHz**: Lower sample rate (training data format)
- **Mono**: Single channel (voices don't need stereo)
- **f32**: Floating-point samples in range [-1.0, 1.0]

### The Resampling Pipeline

**From `whis-core/src/resample.rs`** (conceptual):

```rust
pub fn resample_to_16k(samples: &[f32], sample_rate: u32, channels: u32) -> Result<Vec<f32>> {
    // Step 1: Convert stereo to mono (average channels)
    let mono = if channels == 2 {
        samples.chunks(2)
            .map(|chunk| (chunk[0] + chunk[1]) / 2.0)
            .collect()
    } else {
        samples.to_vec()
    };

    // Step 2: Resample to 16kHz using rubato
    if sample_rate == 16000 {
        return Ok(mono);
    }

    let resampler = rubato::FftFixedIn::<f32>::new(
        sample_rate as usize,  // from
        16000,                 // to
        mono.len(),            // chunk size
        1,                     // channels
    )?;

    let resampled = resampler.process(&[mono], None)?;
    Ok(resampled.into_iter().flatten().collect())
}
```

**Why `rubato`?**
- FFT-based resampling (high quality)
- Pure Rust (no C dependencies beyond whisper itself)
- Handles arbitrary sample rate conversions

## The LocalWhisperProvider

Now let's see how these pieces come together in the provider implementation.

**From `whis-core/src/provider/local_whisper.rs`**:

```rust
pub struct LocalWhisperProvider;

#[async_trait]
impl TranscriptionBackend for LocalWhisperProvider {
    fn name(&self) -> &'static str {
        "local-whisper"
    }

    fn display_name(&self) -> &'static str {
        "Local Whisper"
    }

    fn transcribe_sync(
        &self,
        api_key: &str,  // Actually the model path!
        request: TranscriptionRequest,
    ) -> Result<TranscriptionResult> {
        transcribe_local(api_key, request)
    }

    async fn transcribe_async(
        &self,
        _client: &reqwest::Client,
        api_key: &str,
        request: TranscriptionRequest,
    ) -> Result<TranscriptionResult> {
        let model_path = api_key.to_string();
        let request = request.clone();

        // Run blocking whisper in a separate thread
        tokio::task::spawn_blocking(move || {
            transcribe_local(&model_path, request)
        }).await?
    }
}
```

> **Key Insight**: The `api_key` parameter is repurposed as the model file path. This keeps the `TranscriptionBackend` trait uniform across all providers.

### The Core Transcription Function

```rust
fn transcribe_local(model_path: &str, request: TranscriptionRequest) -> Result<TranscriptionResult> {
    // 1. Validate model exists
    if !std::path::Path::new(model_path).exists() {
        return Err(anyhow!(
            "Whisper model not found at: {}\n\
             Download with: whis setup local",
            model_path
        ));
    }

    // 2. Load whisper model
    let ctx = WhisperContext::new_with_params(
        model_path,
        WhisperContextParameters::default(),
    ).context("Failed to load whisper model")?;

    // 3. Decode MP3 to PCM
    let pcm_samples = decode_and_resample(&request.audio_data)?;

    // 4. Configure whisper parameters
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(request.language.as_deref());
    params.set_print_progress(false);
    params.set_print_timestamps(false);

    // 5. Run transcription
    let mut state = ctx.create_state()?;
    state.full(params, &pcm_samples)?;

    // 6. Extract text from segments
    let num_segments = state.full_n_segments()?;
    let mut text = String::new();
    for i in 0..num_segments {
        if let Ok(segment) = state.full_get_segment_text(i) {
            text.push_str(&segment);
            text.push(' ');
        }
    }

    Ok(TranscriptionResult {
        text: text.trim().to_string(),
    })
}
```

### MP3 Decoding and Resampling

```rust
fn decode_and_resample(mp3_data: &[u8]) -> Result<Vec<f32>> {
    // Decode MP3 using minimp3
    let mut decoder = minimp3::Decoder::new(mp3_data);
    let mut samples = Vec::new();
    let mut sample_rate = 0;
    let mut channels = 0;

    loop {
        match decoder.next_frame() {
            Ok(frame) => {
                sample_rate = frame.sample_rate as u32;
                channels = frame.channels as u32;
                // Convert i16 samples to f32
                for sample in frame.data {
                    samples.push(sample as f32 / 32768.0);
                }
            }
            Err(minimp3::Error::Eof) => break,
            Err(e) => return Err(anyhow!("MP3 decode error: {}", e)),
        }
    }

    // Resample to 16kHz mono
    resample::resample_to_16k(&samples, sample_rate, channels)
}
```

**Why `minimp3`?**
- Pure Rust (no system dependencies)
- Streaming decoder (low memory usage)
- Fast enough for real-time decoding

## The RemoteWhisperProvider

For users who want GPU acceleration or shared infrastructure, the remote provider connects to a self-hosted whisper server.

**From `whis-core/src/provider/remote_whisper.rs`**:

```rust
pub struct RemoteWhisperProvider;

const MODEL: &str = "Systran/faster-whisper-small";

impl RemoteWhisperProvider {
    fn build_api_url(server_url: &str) -> Result<String> {
        // Validate URL format
        if !server_url.starts_with("http://") && !server_url.starts_with("https://") {
            return Err(anyhow!(
                "Invalid server URL: must start with http:// or https://"
            ));
        }

        // Append transcription endpoint
        let base = server_url.trim_end_matches('/');
        Ok(format!("{}/v1/audio/transcriptions", base))
    }
}
```

### OpenAI-Compatible API

The remote whisper server (faster-whisper-server) uses the same API format as OpenAI:

```rust
#[async_trait]
impl TranscriptionBackend for RemoteWhisperProvider {
    fn transcribe_sync(
        &self,
        api_key: &str,  // Actually the server URL!
        request: TranscriptionRequest,
    ) -> Result<TranscriptionResult> {
        let api_url = Self::build_api_url(api_key)?;

        // Reuse the OpenAI-compatible helper
        openai_compatible_transcribe_sync(
            &api_url,
            MODEL,
            "no-auth",  // Self-hosted servers typically don't need auth
            request,
        )
    }
}
```

> **Key Insight**: By using the OpenAI-compatible API, we can reuse the same HTTP/multipart logic for both cloud OpenAI and self-hosted servers.

## Ollama Integration

For polishing transcripts locally, Whis integrates with Ollama - a popular local LLM server.

### The `ollama.rs` Module

**From `whis-core/src/ollama.rs:11-14`**:

```rust
pub const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";
pub const DEFAULT_OLLAMA_MODEL: &str = "ministral-3:3b";
```

### Checking Ollama Status

```rust
/// Check if Ollama is reachable at the given URL
pub fn is_ollama_running(url: &str) -> bool {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .ok();

    if let Some(client) = client {
        let tags_url = format!("{}/api/tags", url.trim_end_matches('/'));
        client.get(&tags_url).send().is_ok()
    } else {
        false
    }
}

/// Check if Ollama binary is installed
pub fn is_ollama_installed() -> bool {
    Command::new("ollama")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}
```

### Auto-Starting Ollama

**From `whis-core/src/ollama.rs:61-134`**:

```rust
pub fn ensure_ollama_running(url: &str) -> Result<bool> {
    // Already running?
    if is_ollama_running(url) {
        return Ok(false);
    }

    // Only auto-start for localhost
    if !url.contains("localhost") && !url.contains("127.0.0.1") {
        return Err(anyhow!(
            "Ollama not reachable at {}. For remote servers, ensure it's running.",
            url
        ));
    }

    // Check if installed
    if !is_ollama_installed() {
        return Err(anyhow!(
            "Ollama is not installed.\n\
             Install from: https://ollama.ai"
        ));
    }

    // Start in background (platform-specific)
    #[cfg(target_os = "linux")]
    {
        Command::new("setsid")
            .args(["ollama", "serve"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("ollama")
            .arg("serve")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
    }

    // Wait for startup (poll every 500ms, timeout 30s)
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(30) {
        if is_ollama_running(url) {
            return Ok(true);  // Started successfully
        }
        std::thread::sleep(Duration::from_millis(500));
    }

    Err(anyhow!("Ollama did not start in time"))
}
```

**Platform differences**:
- **Linux**: Uses `setsid` to detach from terminal (daemon-like)
- **macOS**: Direct spawn (launchd handles backgrounding)
- **Windows**: Not supported for auto-start

### Model Management

```rust
/// Check if a model is available
pub fn has_model(url: &str, model: &str) -> Result<bool> {
    let tags_url = format!("{}/api/tags", url.trim_end_matches('/'));
    let response: TagsResponse = reqwest::blocking::get(&tags_url)?.json()?;

    // Model names can include tags like "phi3:latest"
    let model_base = model.split(':').next().unwrap_or(model);
    Ok(response.models.iter().any(|m| m.name.starts_with(model_base)))
}

/// Pull a model from Ollama registry
pub fn pull_model(_url: &str, model: &str) -> Result<()> {
    eprintln!("Pulling Ollama model '{}'...", model);

    let status = Command::new("ollama")
        .args(["pull", model])
        .status()?;

    if !status.success() {
        return Err(anyhow!("Failed to pull model"));
    }
    Ok(())
}
```

**Why use CLI for pulling?**
The `ollama pull` command shows a nice progress bar. Using the HTTP API would require implementing progress display ourselves.

### Combined Setup Helper

```rust
pub fn ensure_ollama_ready(url: &str, model: &str) -> Result<()> {
    ensure_ollama_running(url)?;

    if !has_model(url, model)? {
        pull_model(url, model)?;
    }

    Ok(())
}
```

This is what the setup wizard calls: "ensure Ollama is running with the required model."

## The Setup Wizard

Whis provides an interactive setup command for easy configuration.

**From `whis-cli/src/commands/setup.rs`**:

### Three Setup Modes

```rust
pub enum SetupMode {
    Cloud,                           // Configure cloud API provider
    Local,                           // Embedded whisper + Ollama
    SelfHosted { url: Option<String> }, // Docker server
}

pub fn run(mode: SetupMode) -> Result<()> {
    match mode {
        SetupMode::Cloud => setup_cloud(),
        SetupMode::Local => setup_local(),
        SetupMode::SelfHosted { url } => setup_self_hosted(url),
    }
}
```

### Local Setup Flow

```rust
fn setup_local() -> Result<()> {
    println!("Local Setup");
    println!("===========");

    // Step 1: Download whisper model
    let model_path = model::default_model_path(model::DEFAULT_MODEL);
    if !model::model_exists(&model_path) {
        model::download_model(model::DEFAULT_MODEL, &model_path)?;
    }

    // Step 2: Setup Ollama
    if !ollama::is_ollama_installed() {
        return Err(anyhow!("Please install Ollama from https://ollama.ai"));
    }

    ollama::ensure_ollama_running(ollama::DEFAULT_OLLAMA_URL)?;

    if !ollama::has_model(ollama::DEFAULT_OLLAMA_URL, ollama::DEFAULT_OLLAMA_MODEL)? {
        ollama::pull_model(ollama::DEFAULT_OLLAMA_URL, ollama::DEFAULT_OLLAMA_MODEL)?;
    }

    // Step 3: Save configuration
    let mut settings = Settings::load();
    settings.provider = TranscriptionProvider::LocalWhisper;
    settings.whisper_model_path = Some(model_path.to_string_lossy().to_string());
    settings.polisher = Polisher::Ollama;
    settings.ollama_url = Some(ollama::DEFAULT_OLLAMA_URL.to_string());
    settings.ollama_model = Some(ollama::DEFAULT_OLLAMA_MODEL.to_string());
    settings.save()?;

    println!("Setup complete!");
    Ok(())
}
```

### Self-Hosted Setup Flow

```rust
fn setup_self_hosted(url_arg: Option<String>) -> Result<()> {
    // Get server URL (prompt if not provided)
    let whisper_url = url_arg.unwrap_or_else(|| {
        prompt_string("Whisper server URL", "http://localhost:8765")
    });

    // Test connectivity
    let health_url = format!("{}/health", whisper_url.trim_end_matches('/'));
    let response = reqwest::blocking::get(&health_url)?;
    if !response.status().is_success() {
        return Err(anyhow!("Whisper server not reachable"));
    }

    // Configure
    let mut settings = Settings::load();
    settings.provider = TranscriptionProvider::RemoteWhisper;
    settings.remote_whisper_url = Some(whisper_url);
    settings.polisher = Polisher::Ollama;
    settings.save()?;

    Ok(())
}
```

## Docker Deployment

For self-hosted setups, Whis includes Docker Compose configurations.

### GPU Stack (`docker/docker-compose.yml`)

```yaml
services:
  whisper:
    image: fedirz/faster-whisper-server:latest-cuda
    container_name: whis-whisper
    ports:
      - "8765:8000"
    environment:
      - WHISPER__MODEL=Systran/faster-whisper-small
    deploy:
      resources:
        reservations:
          devices:
            - capabilities: [gpu]

  ollama:
    image: ollama/ollama:latest
    container_name: whis-ollama
    ports:
      - "11434:11434"
    volumes:
      - ollama_data:/root/.ollama
    deploy:
      resources:
        reservations:
          devices:
            - capabilities: [gpu]
```

### CPU Stack (`docker/docker-compose.cpu.yml`)

```yaml
services:
  whisper:
    image: fedirz/faster-whisper-server:latest
    environment:
      - WHISPER__MODEL=Systran/faster-whisper-tiny  # Smaller model for CPU
    # No GPU reservation

  ollama:
    image: ollama/ollama:latest
    # No GPU reservation
```

### Starting the Server

```bash
# GPU (requires NVIDIA Container Toolkit)
cd docker && docker compose up -d

# CPU-only
cd docker && docker compose -f docker-compose.cpu.yml up -d

# Pull polish model
docker exec -it whis-ollama ollama pull phi3
```

## Configuration Fields

The `Settings` struct includes these local transcription fields:

```rust
pub struct Settings {
    // ... existing fields ...

    /// Path to whisper model file (for local-whisper)
    #[serde(default)]
    pub whisper_model_path: Option<String>,

    /// URL for self-hosted whisper server (for remote-whisper)
    #[serde(default)]
    pub remote_whisper_url: Option<String>,

    /// Ollama server URL
    #[serde(default)]
    pub ollama_url: Option<String>,

    /// Ollama model for polishing
    #[serde(default)]
    pub ollama_model: Option<String>,
}
```

**Environment variable fallbacks**:

| Setting | Config Flag | Environment Variable |
|---------|-------------|---------------------|
| Model path | `--whisper-model-path` | `WHISPER_MODEL_PATH` |
| Server URL | `--remote-whisper-url` | `REMOTE_WHISPER_URL` |
| Ollama URL | `--ollama-url` | `OLLAMA_URL` |
| Ollama model | `--ollama-model` | `OLLAMA_MODEL` |

## Design Decisions

### 1. Why Not Embed llama.cpp for Polishing?

**The problem**: Both whisper.cpp and llama.cpp bundle ggml (a tensor library), causing duplicate symbol errors at link time:

```
error: duplicate symbol '_ggml_init' in:
    whisper.o
    llama.o
```

**Solution**: Use Ollama via HTTP instead of embedding llama.cpp.

**Benefits**:
- Avoids complex build system hacks
- Users can choose their own polish model
- Ollama is already popular for local LLMs
- HTTP API is simpler than FFI

### 2. Why Auto-Start Ollama?

Users expect "it just works." Having to manually run `ollama serve` before every transcription is friction. Auto-starting removes that friction for localhost usage.

### 3. Why Feature Flag for Binary Size?

The `local-whisper` feature adds ~45MB to the binary. Users who only need cloud transcription shouldn't pay this cost:

```bash
# Cloud-only build (smaller)
cargo build --no-default-features --features ffmpeg,clipboard

# Full build (larger)
cargo build  # local-whisper is in defaults
```

### 4. Why faster-whisper-server for Remote?

- **4x faster** than whisper.cpp (uses CTranslate2)
- **OpenAI-compatible API** (reuse existing code)
- **Official Docker images** with GPU support
- **No custom server** code to maintain

## Summary

**Key Takeaways:**

1. **Two approaches**: Embedded (local-whisper) vs Server (remote-whisper)
2. **Model management**: Download from HuggingFace, store in platform-specific location
3. **Audio pipeline**: MP3 decode (minimp3) → Resample to 16kHz (rubato) → Transcribe (whisper-rs)
4. **Ollama integration**: Auto-start, model pulling, HTTP API for polish
5. **Setup wizard**: `whis setup local` or `whis setup self-hosted`

**Where This Matters in Whis:**

- Provider registration: `whis-core/src/provider/mod.rs`
- Local whisper: `whis-core/src/provider/local_whisper.rs`
- Remote whisper: `whis-core/src/provider/remote_whisper.rs`
- Model download: `whis-core/src/model.rs`
- Ollama management: `whis-core/src/ollama.rs`
- Audio resampling: `whis-core/src/resample.rs`
- Setup wizard: `whis-cli/src/commands/setup.rs`
- Docker configs: `docker/docker-compose*.yml`

**Patterns Used:**

- **Feature flags**: Conditional compilation for binary size
- **Trait object reuse**: Same `TranscriptionBackend` for local and cloud
- **Parameter repurposing**: `api_key` field holds model path or server URL
- **Auto-recovery**: Start Ollama if not running
- **Temp file pattern**: Download to `.tmp`, rename on success

---

Next: [Chapter 15: Parallel Transcription](./ch15-parallel.md)
