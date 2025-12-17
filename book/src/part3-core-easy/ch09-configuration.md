# Chapter 9: Configuration & Settings

Every application needs configuration: API keys, user preferences, default values. Whis stores all settings in a single JSON file at `~/.config/whis/settings.json`. This chapter explores how Whis uses `serde` for serialization, implements defaults, and handles environment variable fallbacks.

## The Settings Struct

Let's start with the core data structure from `whis-core/src/settings.rs:10-42`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub shortcut: String,
    #[serde(default)]
    pub provider: TranscriptionProvider,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub api_keys: HashMap<String, String>,
    #[serde(default)]
    pub polisher: Polisher,
    #[serde(default)]
    pub polish_prompt: Option<String>,
    #[serde(default)]
    pub whisper_model_path: Option<String>,
    #[serde(default)]
    pub ollama_url: Option<String>,
    #[serde(default)]
    pub ollama_model: Option<String>,
    #[serde(default)]
    pub remote_whisper_url: Option<String>,
}
```

**Key observations**:

1. **`#[derive(Serialize, Deserialize)]`**: Enables JSON conversion with `serde_json`
2. **`#[serde(default)]`**: Missing fields use `Default::default()` instead of failing deserialization
3. **`Option<String>`**: Fields can be `None` (not configured)
4. **`HashMap<String, String>`**: API keys stored as `"openai" -> "sk-..."`

> **Key Insight**: The `#[serde(default)]` attribute is crucial for backward compatibility. When you add new fields to `Settings`, old config files (missing those fields) still deserialize successfully using the field's default value.

## Example Settings File

Here's what `~/.config/whis/settings.json` looks like:

```json
{
  "shortcut": "Ctrl+Shift+R",
  "provider": "openai",
  "language": "en",
  "api_keys": {
    "openai": "sk-proj-abc123...",
    "groq": "gsk_xyz789..."
  },
  "polisher": "openai",
  "polish_prompt": null,
  "whisper_model_path": null,
  "ollama_url": "http://localhost:11434",
  "ollama_model": "ministral-3:3b",
  "remote_whisper_url": null
}
```

**Reading this**:
- User prefers OpenAI for transcription
- Has API keys for OpenAI and Groq
- Uses OpenAI for polishing transcripts
- Language hint: English (`"en"`)
- Ollama configured for local LLM polishing
- No local whisper model configured (using cloud)

> **Note**: For local transcription setup, see [Chapter 14b: Local Transcription](../part4-core-advanced/ch14b-local-transcription.md).

## Default Values

When no config file exists, or fields are missing, Whis uses sensible defaults:

```rust
impl Default for Settings {
    fn default() -> Self {
        Self {
            shortcut: "Ctrl+Shift+R".to_string(),
            provider: TranscriptionProvider::default(), // OpenAI
            language: None, // Auto-detect
            api_keys: HashMap::new(),
            polisher: Polisher::default(), // None
            polish_prompt: None,
            whisper_model_path: None,
            ollama_url: None,
            ollama_model: None,
            remote_whisper_url: None,
        }
    }
}
```

**From `whis-core/src/settings.rs:44-59`**

**Why these defaults?**
- `"Ctrl+Shift+R"`: Unlikely to conflict with other apps
- `OpenAI`: Most popular and reliable provider
- `None` for polisher: Don't add latency/cost by default
- Auto-detect language: Works for most users

## The Provider Enum

`TranscriptionProvider` is an enum that serde serializes to lowercase strings:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TranscriptionProvider {
    #[default]
    OpenAI,
    Mistral,
    Groq,
    Deepgram,
    ElevenLabs,
    #[serde(rename = "local-whisper")]
    LocalWhisper,
    #[serde(rename = "remote-whisper")]
    RemoteWhisper,
}
```

**From `whis-core/src/config.rs:7-18`**

**Serde attributes explained**:

1. **`#[serde(rename_all = "lowercase")]`**: 
   - JSON: `"openai"`, `"mistral"`, `"groq"` (not `"OpenAI"`, `"Mistral"`)
   
2. **`#[serde(rename = "local-whisper")]`**: 
   - Override for `LocalWhisper` → JSON: `"local-whisper"`
   - Without this: would be `"localwhisper"` (lowercase conversion)

3. **`#[default]`**: 
   - When deserializing fails or field is missing, use `OpenAI`

**Provider string conversions**:

```rust
impl TranscriptionProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            TranscriptionProvider::OpenAI => "openai",
            TranscriptionProvider::Groq => "groq",
            // ... etc
        }
    }
}
```

**From `whis-core/src/config.rs:22-32`**

This lets you convert between enum variants and strings for JSON/CLI.

## Loading Settings from Disk

The `load()` method reads the config file, or falls back to defaults:

```rust
pub fn load() -> Self {
    let path = Self::path(); // ~/.config/whis/settings.json
    if let Ok(content) = fs::read_to_string(&path)
        && let Ok(settings) = serde_json::from_str(&content)
    {
        return settings;
    }
    Self::default()
}
```

**From `whis-core/src/settings.rs:141-149`**

**Let-chain pattern** (`if let ... && let ...`):
- First: Try to read file as string
- Then: Try to parse JSON
- If either fails: Return default settings
- No panics, no unwraps

**Why not `?` operator?**  
Because we want to return defaults on error, not propagate the error up. This is a "load or use defaults" pattern, not a "load or fail" pattern.

## Saving Settings to Disk

Writing settings back creates parent directories and sets restrictive permissions:

```rust
pub fn save(&self) -> Result<()> {
    let path = Self::path();
    
    // Create ~/.config/whis/ if it doesn't exist
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    // Serialize to pretty JSON
    let content = serde_json::to_string_pretty(self)?;
    fs::write(&path, &content)?;

    // On Unix: chmod 600 (only owner can read/write)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}
```

**From `whis-core/src/settings.rs:152-167`**

**Security note**: API keys are sensitive. Setting permissions to `0o600` (owner read/write only) prevents other users on the system from reading your keys.

**Platform differences**:
- **Unix/Linux/macOS**: `chmod 600` applied
- **Windows**: Uses default ACLs (typically restricts to current user)

## API Key Management

API keys can come from two places: the config file or environment variables.

```rust
pub fn get_api_key(&self) -> Option<String> {
    self.get_api_key_for(&self.provider)
}

pub fn get_api_key_for(&self, provider: &TranscriptionProvider) -> Option<String> {
    // 1. Check api_keys map in config
    if let Some(key) = self.api_keys.get(provider.as_str())
        && !key.is_empty()
    {
        return Some(key.clone());
    }

    // 2. Fall back to environment variable
    std::env::var(provider.api_key_env_var()).ok()
}
```

**From `whis-core/src/settings.rs:71-90`**

**Priority order**:
1. Config file `api_keys` map
2. Environment variable (e.g., `OPENAI_API_KEY`)

**Why this design?**
- Config file: Persistent, survives restarts, user-friendly
- Environment variables: CI/CD, Docker, temporary overrides

**Environment variable mapping**:

```rust
pub fn api_key_env_var(&self) -> &'static str {
    match self {
        TranscriptionProvider::OpenAI => "OPENAI_API_KEY",
        TranscriptionProvider::Groq => "GROQ_API_KEY",
        TranscriptionProvider::LocalWhisper => "WHISPER_MODEL_PATH",
        // ... etc
    }
}
```

**From `whis-core/src/config.rs:35-45`**

> **Key Insight**: Local providers use different env vars. `WHISPER_MODEL_PATH` is a file path, not an API key. This same pattern (config → env fallback) applies to all providers.

### Local Transcription Settings

For local and self-hosted transcription, additional settings control where to find models and servers:

```rust
// Local whisper model path
pub whisper_model_path: Option<String>,

// Self-hosted whisper server URL
pub remote_whisper_url: Option<String>,

// Ollama server for local polishing
pub ollama_url: Option<String>,
pub ollama_model: Option<String>,
```

**Environment variable mapping**:

| Setting | Config Flag | Env Var | Default |
|---------|-------------|---------|---------|
| Model path | `--whisper-model-path` | `WHISPER_MODEL_PATH` | None |
| Server URL | `--remote-whisper-url` | `REMOTE_WHISPER_URL` | None |
| Ollama URL | `--ollama-url` | `OLLAMA_URL` | `http://localhost:11434` |
| Ollama model | `--ollama-model` | `OLLAMA_MODEL` | `ministral-3:3b` |

See [Chapter 14b: Local Transcription](../part4-core-advanced/ch14b-local-transcription.md) for setup instructions.

## Configuration File Path

Where does the config file live?

```rust
pub fn path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("whis")
        .join("settings.json")
}
```

**From `whis-core/src/settings.rs:63-68`**

**Using the `dirs` crate**:
- **Linux**: `~/.config/whis/settings.json`
- **macOS**: `~/Library/Application Support/whis/settings.json`
- **Windows**: `C:\Users\<name>\AppData\Roaming\whis\settings.json`
- **Fallback**: `./whis/settings.json` (current directory)

This follows platform conventions automatically.

## The Polisher Enum

Polishing is optional transcript cleanup using LLMs:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Polisher {
    #[default]
    None,      // No polishing
    OpenAI,    // Uses GPT models
    Mistral,   // Uses Mistral models
    Ollama,    // Local LLM via Ollama
}
```

**From `whis-core/src/polish.rs:17-23`**

**Usage pattern**:

```rust
pub fn get_polisher_api_key(&self) -> Option<String> {
    match &self.polisher {
        Polisher::None | Polisher::Ollama => None,  // No API key needed
        Polisher::OpenAI => self.get_api_key_for(&TranscriptionProvider::OpenAI),
        Polisher::Mistral => self.get_api_key_for(&TranscriptionProvider::Mistral),
    }
}
```

**From `whis-core/src/settings.rs:104-110`**

**Why `Ollama` returns `None`?**  
Ollama runs locally and doesn't need API keys. It uses `ollama_url` instead (e.g., `http://localhost:11434`).

## The Default Polish Prompt

When polishing is enabled, Whis sends the transcript through an LLM with this prompt:

```rust
pub const DEFAULT_POLISH_PROMPT: &str = 
    "Clean up this voice transcript. Fix grammar and punctuation. \
     Remove filler words (um, uh, like, you know). \
     If the speaker corrects themselves, keep only the correction. \
     Preserve technical terms and proper nouns. Output only the cleaned text.";
```

**From `whis-core/src/polish.rs:9-12`**

Users can override this via `settings.polish_prompt`.

## Real-World Usage Example

Here's how the desktop app loads and uses settings:

```rust
// On startup
let settings = Settings::load();

// Check if API key exists
if !settings.has_api_key() {
    // Show "Configure API Key" prompt
}

// Get API key for current provider
let api_key = settings.get_api_key()
    .context("No API key configured")?;

// Transcribe audio
let transcript = provider.transcribe(&audio_data, &api_key).await?;

// Polish if enabled
let final_text = if settings.polisher != Polisher::None {
    let polish_key = settings.get_polisher_api_key()
        .context("No polisher API key")?;
    let prompt = settings.polish_prompt
        .as_deref()
        .unwrap_or(DEFAULT_POLISH_PROMPT);
    polish(&transcript, &settings.polisher, &polish_key, prompt, None).await?
} else {
    transcript
};

// Copy to clipboard
clipboard::copy(&final_text)?;
```

This shows the full flow: load → validate → transcribe → polish → clipboard.

## Summary

**Key Takeaways:**

1. **Settings struct**: Single source of truth for all configuration
2. **Serde attributes**: `#[serde(default)]` enables backward compatibility
3. **Priority**: Config file → Environment variables → Defaults
4. **Security**: `chmod 600` on Unix to protect API keys
5. **Platform paths**: `dirs` crate handles platform differences

**Where This Matters in Whis:**

- CLI loads settings on every command (`whis-cli/src/commands/`)
- Desktop GUI loads once at startup (`whis-desktop/src/state.rs`)
- Settings panel allows editing and saving (`whis-desktop/ui/src/views/Settings.svelte`)

**Design Patterns Used:**

- **Builder pattern**: `Settings::default()` provides safe starting point
- **Fallback chain**: Config → Env → Default
- **Newtype pattern**: `TranscriptionProvider` enum wraps strings safely
- **Explicit defaults**: `#[default]` and `impl Default` make intent clear

**What We Skipped:**

- How to validate settings (e.g., checking API key format)
- Migrations when settings schema changes
- Handling corrupt JSON files gracefully

Those are implementation details you can explore in the actual code if needed.

---

Next: [Chapter 10: Clipboard Operations](./ch10-clipboard.md)
