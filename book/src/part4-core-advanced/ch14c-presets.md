# Chapter 14c: The Preset System

Transcription is just the first step. Often you need the output formatted for a specific purpose: an AI prompt, an email, personal notes. The preset system lets you define reusable output formats that transform raw transcripts into polished, purpose-specific text.

## What Are Presets?

A preset is a named configuration that tells the polisher *how* to transform your transcript. Each preset contains:

1. **Description**: What this preset is for
2. **Prompt**: System prompt for the LLM
3. **Overrides** (optional): Different polisher or model

**Example**: The `email` preset transforms rambling voice notes into concise emails. The `ai-prompt` preset cleans up voice input for pasting into AI assistants.

## The Preset Struct

**From `whis-core/src/preset.rs:8-27`**:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    /// Unique identifier (derived from filename, not serialized)
    #[serde(skip)]
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// The system prompt for the LLM
    pub prompt: String,

    /// Optional: Override the polisher for this preset (openai, mistral)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polisher: Option<String>,

    /// Optional: Override the model for this preset
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}
```

**Key observations**:

1. **`#[serde(skip)]` on name**: The preset name comes from the filename, not the JSON content
2. **`skip_serializing_if`**: Optional fields are omitted from JSON when `None` (cleaner files)
3. **Polisher/model overrides**: Advanced users can specify a different backend per preset

## Preset Sources

Presets come from two places:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetSource {
    BuiltIn,  // Hardcoded in the binary
    User,     // Loaded from ~/.config/whis/presets/
}
```

**From `whis-core/src/preset.rs:29-34`**

**Loading precedence**: User presets override built-in presets with the same name. This lets you customize the defaults without losing them.

## Built-In Presets

Whis ships with three built-in presets:

### 1. `ai-prompt`

Optimized for pasting into AI assistants:

```rust
Preset {
    name: "ai-prompt".to_string(),
    description: "Clean transcript for AI assistant prompts".to_string(),
    prompt: "Clean up this voice transcript for use as an AI assistant prompt. \
        Fix grammar and punctuation. Remove filler words. \
        Keep it close to plain text, but use minimal markdown when it improves clarity: \
        lists (ordered/unordered) for multiple items, bold for emphasis, headings only when absolutely necessary. \
        Preserve the speaker's intent and technical terminology. \
        Output only the cleaned text.".to_string(),
    polisher: None,
    model: None,
}
```

**From `whis-core/src/preset.rs:57-69`**

### 2. `email`

Transforms voice notes into email format:

```rust
Preset {
    name: "email".to_string(),
    description: "Format transcript as an email".to_string(),
    prompt: "Clean up this voice transcript into an email. \
        Fix grammar and punctuation. Remove filler words. \
        Keep it concise. Match the sender's original tone (casual or formal). \
        Do NOT add placeholder names or unnecessary formalities. \
        Output only the cleaned text.".to_string(),
    polisher: None,
    model: None,
}
```

**From `whis-core/src/preset.rs:70-81`**

### 3. `notes`

Light cleanup for personal notes:

```rust
Preset {
    name: "notes".to_string(),
    description: "Light cleanup for personal notes".to_string(),
    prompt: "Lightly clean up this voice transcript for personal notes. \
        Fix major grammar issues and remove excessive filler words. \
        Preserve the speaker's natural voice and thought structure. \
        IMPORTANT: Start directly with the cleaned content. NEVER add any introduction, preamble, or meta-commentary like 'Here are the notes'. \
        Output ONLY the cleaned transcript, nothing else.".to_string(),
    polisher: None,
    model: None,
}
```

**From `whis-core/src/preset.rs:82-94`**

## User Preset Storage

User presets are JSON files stored in:

```rust
pub fn presets_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("whis")
        .join("presets")
}
```

**From `whis-core/src/preset.rs:46-52`**

**Platform paths**:
- **Linux**: `~/.config/whis/presets/`
- **macOS**: `~/Library/Application Support/whis/presets/`
- **Windows**: `C:\Users\<name>\AppData\Roaming\whis\presets\`

Each preset is a `.json` file named after the preset: `my-preset.json` creates a preset named `my-preset`.

## Example User Preset

Here's a complete user preset file (`~/.config/whis/presets/code-review.json`):

```json
{
  "description": "Format transcript as code review feedback",
  "prompt": "Transform this voice transcript into structured code review feedback. Fix grammar. Organize into sections: Summary, Issues Found, Suggestions. Use bullet points for clarity. Be constructive but direct. Output only the formatted review.",
  "polisher": "openai",
  "model": "gpt-4o"
}
```

**Note**: The `name` field is not included—it's derived from the filename.

## Loading Presets

The `load()` method implements the precedence logic:

```rust
pub fn load(name: &str) -> Result<(Preset, PresetSource), String> {
    // Check user presets first
    if let Some(preset) = Self::load_user_preset(name) {
        return Ok((preset, PresetSource::User));
    }

    // Fall back to built-in
    if let Some(preset) = Self::builtins().into_iter().find(|p| p.name == name) {
        return Ok((preset, PresetSource::BuiltIn));
    }

    Err(format!(
        "Unknown preset '{}'\nAvailable: {}",
        name,
        Self::all_names().join(", ")
    ))
}
```

**From `whis-core/src/preset.rs:124-140`**

**Priority order**:
1. User preset file (`~/.config/whis/presets/{name}.json`)
2. Built-in preset
3. Error with available options

## Name Validation

Preset names must follow these rules:

```rust
pub fn validate_name(name: &str, allow_builtin_conflict: bool) -> Result<(), String> {
    let name = name.trim();

    if name.is_empty() {
        return Err("Preset name cannot be empty".to_string());
    }

    if name.len() > 50 {
        return Err("Preset name must be 50 characters or less".to_string());
    }

    // Check valid characters
    if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err(
            "Preset name can only contain letters, numbers, hyphens, and underscores".to_string(),
        );
    }

    // Check for built-in conflict
    if !allow_builtin_conflict && Self::is_builtin(name) {
        return Err(format!(
            "Cannot use '{}' - it's a built-in preset name",
            name
        ));
    }

    Ok(())
}
```

**From `whis-core/src/preset.rs:213-244`**

**Rules**:
- 1-50 characters
- Alphanumeric, hyphens, underscores only
- No spaces or special characters
- Built-in names are reserved (unless intentionally overriding)

## CLI Commands

### List All Presets

```bash
$ whis presets
NAME       SOURCE    DESCRIPTION
ai-prompt  built-in  Clean transcript for AI assistant prompts
email      built-in  Format transcript as an email
notes      built-in  Light cleanup for personal notes

User presets: /home/user/.config/whis/presets/
```

**From `whis-cli/src/commands/presets.rs:15-50`**

### Show Preset Details

```bash
$ whis presets show email
Preset: email (built-in)

Description:
  Format transcript as an email

Prompt:
  Clean up this voice transcript into an email. Fix grammar and punctuation.
  Remove filler words. Keep it concise. Match the sender's original tone
  (casual or formal). Do NOT add placeholder names or unnecessary formalities.
  Output only the cleaned text.
```

### Create New Preset Template

```bash
$ whis presets new code-review
{
  "description": "Describe what this preset does",
  "prompt": "Your system prompt here"
}

Save to: /home/user/.config/whis/presets/code-review.json
```

**From `whis-cli/src/commands/presets.rs:91-103`**

### Edit Preset in Editor

```bash
$ whis presets edit my-notes
Created new preset: /home/user/.config/whis/presets/my-notes.json
# Opens in $EDITOR (falls back to nano)
Preset saved: /home/user/.config/whis/presets/my-notes.json
```

**From `whis-cli/src/commands/presets.rs:105-130`**

If the preset doesn't exist, it creates a template file first.

## Using Presets

### With `--as` Flag

```bash
# Record and apply preset
$ whis --as email
🎤 Recording... Press Enter to stop.
⏹️  Stopped.
🔄 Transcribing...
✨ Applying email preset...
✅ [polished email output]
📋 Copied to clipboard.
```

### With Daemon

```bash
# Configure default preset in settings
$ whis config --active-preset email

# Now all recordings use this preset
$ whis listen
```

## Preset vs Polish

| Feature | `--polish` | `--as <preset>` |
|---------|-----------|-----------------|
| Uses | Default polish prompt | Preset's custom prompt |
| Customization | None | Full prompt control |
| Model override | No | Yes (per preset) |
| Polisher override | No | Yes (per preset) |

**When to use which?**

- `--polish`: Quick cleanup, default behavior
- `--as <preset>`: Specific output format, custom prompts

## Advanced: Polisher Overrides

Presets can override the polisher and model. This is useful when:

1. **Quality needs differ**: Use GPT-4o for important emails, local Ollama for quick notes
2. **Cost control**: Route expensive presets to cloud, cheap ones to local
3. **Speed optimization**: Fast local model for drafts, slower cloud for final output

**Example preset with overrides**:

```json
{
  "description": "High-quality document editing",
  "prompt": "...",
  "polisher": "openai",
  "model": "gpt-4o"
}
```

When this preset is used, it ignores the global polisher setting and uses OpenAI's GPT-4o instead.

## Implementation: Listing All Presets

The `list_all()` method merges built-in and user presets:

```rust
pub fn list_all() -> Vec<(Preset, PresetSource)> {
    let mut presets: HashMap<String, (Preset, PresetSource)> = HashMap::new();

    // Add built-ins first
    for preset in Self::builtins() {
        presets.insert(preset.name.clone(), (preset, PresetSource::BuiltIn));
    }

    // Add user presets (overwrite built-ins if same name)
    if let Ok(entries) = fs::read_dir(Self::presets_dir()) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                // ... parse and insert with PresetSource::User
            }
        }
    }

    // Sort by name
    let mut result: Vec<_> = presets.into_values().collect();
    result.sort_by(|a, b| a.0.name.cmp(&b.0.name));
    result
}
```

**From `whis-core/src/preset.rs:143-186`**

**Algorithm**:
1. Start with built-ins in HashMap
2. Read user presets directory
3. Parse each `.json` file
4. Insert with `PresetSource::User` (overwrites built-in if same name)
5. Sort alphabetically

## Summary

**Key Takeaways:**

1. **Presets = named output formats**: Custom prompts for different use cases
2. **Two sources**: Built-in (hardcoded) and user (`~/.config/whis/presets/`)
3. **User overrides built-in**: Same-name user preset takes precedence
4. **Optional overrides**: Presets can specify different polisher/model
5. **CLI management**: `whis presets list/show/new/edit`

**Where This Matters in Whis:**

- Preset module: `whis-core/src/preset.rs`
- CLI commands: `whis-cli/src/commands/presets.rs`
- CLI args: `whis-cli/src/args.rs` (PresetsAction enum)
- Record-once: `whis-cli/src/commands/record_once.rs` (applies preset)
- Desktop UI: `whis-desktop/ui/src/views/PresetsView.vue`

**Patterns Used:**

- **Precedence chain**: User → Built-in → Error
- **Filename as ID**: Name derived from file, not content
- **HashMap merge**: Built-ins + user files, deduplicated by name
- **Optional overrides**: `Option<String>` for polisher/model

**Design Decisions:**

1. **Why filename as name?** Prevents name/file mismatch, simpler file management
2. **Why allow overriding built-ins?** Users can customize defaults without losing them
3. **Why per-preset polisher?** Different quality/cost tradeoffs per use case
4. **Why JSON?** Human-readable, easy to edit, widely understood

---

Next: [Chapter 15: Parallel Transcription](./ch15-parallel.md)
