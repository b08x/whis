# Whis — Developer Handoff Document

*Strategic context, competitive analysis, and implementation roadmap*

**Last Updated:** December 2024
**Status:** Ready for implementation
**Fact-Checked:** December 2024

---

## Why This Document Exists

This document captures the strategic thinking behind Whis's next major feature: **LLM post-processing**. It combines:

1. A complete product overview for competitive positioning
2. Deep competitive analysis findings from the voice-to-text market
3. A prioritized feature roadmap with implementation details
4. Specific code-level guidance for the recommended first feature

The goal is to provide any developer (including future-you) with full context to continue this work without losing the strategic rationale.

---

## The Strategic Insight

**"Transcription is now table stakes—post-processing pipelines are the new battleground."**

The voice-to-text market has shifted. Wispr Flow raised $81M not for better transcription, but for their "voice OS" vision where speech is transformed into context-aware, structured output. Superwhisper's success comes from its "modes" system. Otter.ai now has AI agents that speak in meetings.

Whis currently does one thing well: **transcribe and copy to clipboard**. But competitors are racing ahead with LLM-powered cleanup, formatting, and transformation. The competitive research reveals that Whis has a unique position to exploit:

| Competitor | Weakness Whis Can Exploit |
|------------|---------------------------|
| **Wispr Flow** | Cloud-only, closed-source, $15/mo subscription |
| **Superwhisper** | macOS-only (Windows beta), GUI-only, no CLI |
| **HyperWhisper** | macOS-only, no Linux or Windows |
| **Otter.ai** | Cloud-only, meeting-focused, no CLI |

**Whis is unique in combining:** open-source Rust implementation, API-based transcription (no local GPU), multi-provider support (OpenAI + Mistral), and first-class Linux support with global hotkeys. Adding LLM post-processing would match the core value of $81M-funded competitors while maintaining the developer-focused, privacy-respecting, terminal-native identity.

---

## Executive Summary

**Whis** is an open-source voice-to-text transcription tool designed specifically for AI-powered workflows. Built in Rust for performance and reliability, it offers both a command-line interface for developers and a native desktop application for everyday users. The core value proposition is elegantly simple: **"Your voice, piped to clipboard."**

**Tagline:** Speak. Paste. Ship.

**Website:** https://whis.ink
**Repository:** https://github.com/frankdierolf/whis
**License:** MIT
**Current Version:** 0.5.9
**Author:** Frank Dierolf

---

## Market Positioning

### Target Audience
- **Primary:** Developers and power users who interact with AI coding assistants (Claude, GitHub Copilot, ChatGPT)
- **Secondary:** Linux desktop users seeking a lightweight, privacy-respecting voice transcription tool
- **Tertiary:** Users who prefer API-based transcription over local models (cost-efficiency over offline capability)

### Core Value Propositions

| Value | Description |
|-------|-------------|
| **AI Workflow Optimized** | Purpose-built for speaking prompts and pasting into AI tools |
| **Cost Efficient** | $0.001-0.006/minute via cloud APIs — no expensive local GPU required |
| **Minimalist Philosophy** | Record → Transcribe → Clipboard. No bloat. |
| **Multi-Provider** | Choice between OpenAI Whisper and Mistral Voxtral |
| **Open Source** | MIT licensed, fully transparent, community-driven |
| **Native Performance** | Built in Rust with minimal runtime overhead |

---

## Product Suite

Whis is architected as a workspace of four interconnected crates, enabling code reuse while supporting diverse deployment targets:

```
whis/
├── whis-core      → Shared library (audio, transcription, clipboard)
├── whis-cli       → Terminal application
├── whis-desktop   → System tray GUI (Tauri + Vue.js)
└── whis-mobile    → Mobile application (in development)
```

### 1. Whis CLI

**Description:** A lightweight command-line tool for terminal-native workflows.

**Installation:**
```bash
cargo install whis
```

**Key Features:**
- **One-shot mode** — Single command to record, transcribe, and copy
- **Background service** — Daemon with global hotkey support
- **Configurable hotkeys** — Default `Ctrl+Shift+R`, fully customizable
- **Multi-provider support** — Switch between OpenAI and Mistral
- **Language hints** — ISO-639-1 codes for improved accuracy

**Commands:**
```bash
whis                          # One-shot recording (Enter to stop)
whis listen                   # Start background service
whis listen -k "super+space"  # Custom hotkey
whis status                   # Check service status
whis stop                     # Stop service
whis config --show            # View configuration
whis config --provider mistral
whis config --language en
```

**Supported Platforms:**
- Linux (x86_64, ARM64) — X11 & Wayland
- macOS (Intel & Apple Silicon)
- Windows (partial — global hotkeys in development)

### 2. Whis Desktop

**Description:** A native desktop application with system tray integration for non-terminal users.

**Technology Stack:**
- **Backend:** Tauri 2 (Rust)
- **Frontend:** Vue.js 3.6
- **System Integration:** GTK for Linux, native on other platforms

**Key Features:**
- **System tray** — Always-on access with visual recording status
- **Global shortcuts** — Works from any application
- **Settings UI** — Graphical configuration for API keys, shortcuts, and provider
- **Visual feedback** — Tray icon changes state (idle → recording → transcribing)
- **Auto-start support** — Launch on login via desktop entry

**Distribution Formats:**
| Format | Method |
|--------|--------|
| **Flatpak** | `flatpak install flathub ink.whis.Whis` (Recommended) |
| **AppImage** | Download from GitHub Releases |
| **Debian/Ubuntu** | `.deb` package |
| **Fedora/RHEL** | `.rpm` package |
| **AUR** | `whis-bin` (Arch Linux) |

### 3. Whis Mobile (In Development)

**Description:** Mobile companion app for iOS and Android.

**Technology Stack:**
- Tauri 2 Mobile
- Embedded MP3 encoder (no FFmpeg dependency)
- Vue.js shared UI

---

## Core Technical Features

### Audio Processing

| Feature | Implementation |
|---------|----------------|
| **Capture** | CPAL (Cross-Platform Audio Library) |
| **Sample Formats** | F32, I16, U16 (auto-detected) |
| **Sample Rate** | Device default (typically 44.1kHz or 48kHz) |
| **Channels** | Mono (optimal for speech) |
| **Encoding** | MP3 @ 128kbps |
| **Encoder (Desktop)** | FFmpeg (libmp3lame) |
| **Encoder (Mobile)** | Embedded LAME (mp3lame_encoder crate) |

### Intelligent Chunking

For long recordings that exceed API limits:

| Parameter | Value |
|-----------|-------|
| **Threshold** | 20 MB file size |
| **Chunk Duration** | 5 minutes |
| **Overlap** | 2 seconds |
| **Purpose** | Prevents word-cutting at chunk boundaries |

### Transcription Engine

**Supported Providers:**

| Provider | Model | Endpoint | Cost |
|----------|-------|----------|------|
| **OpenAI** | `whisper-1` | `api.openai.com/v1/audio/transcriptions` | ~$0.006/min |
| **Mistral** | `voxtral-mini-latest` | `api.mistral.ai/v1/audio/transcriptions` | ~$0.001/min |

> **Cost Note:** Mistral Voxtral is **6x cheaper** than OpenAI Whisper with comparable accuracy. This makes Whis's multi-provider support a significant cost advantage.

### Cost Comparison

| Provider | Cost/minute | 1 hour | 10 hours/month |
|----------|-------------|--------|----------------|
| OpenAI Whisper | $0.006 | $0.36 | $3.60 |
| Mistral Voxtral | $0.001 | $0.06 | $0.60 |

**Recommendation:** Default to Mistral for cost-conscious users; OpenAI for users who need specific Whisper features.

**Advanced Capabilities:**
- **Parallel transcription** — Up to 3 concurrent API requests (semaphore-controlled)
- **Smart overlap merging** — Case-insensitive word-level deduplication (up to 15 words)
- **Language hints** — ISO-639-1 codes (en, de, fr, es, etc.) for improved accuracy
- **Timeout handling** — 5-minute timeout per chunk with graceful error aggregation

### Clipboard Integration

| Platform | Method |
|----------|--------|
| **Linux (X11)** | arboard crate |
| **Linux (Wayland)** | wlr-data-control protocol |
| **Flatpak** | Bundled wl-copy (works around sandbox limitations) |
| **macOS/Windows** | arboard crate (native APIs) |

### Global Hotkeys

| Platform | Implementation |
|----------|----------------|
| **Linux** | rdev crate (raw device events) |
| **macOS/Windows** | global-hotkey crate (Tauri-maintained) |
| **Wayland Desktop** | XDG Portal + fallback to CLI service |

**Linux Setup (one-time for CLI hotkey mode):**
```bash
sudo usermod -aG input $USER
echo 'KERNEL=="uinput", GROUP="input", MODE="0660"' | sudo tee /etc/udev/rules.d/99-uinput.rules
sudo udevadm control --reload-rules && sudo udevadm trigger
# Logout and login
```

---

## Configuration & Settings

**Storage Location:** `~/.config/whis/settings.json`
**File Permissions:** 0600 (read/write owner only)

**Configuration Schema:**
```json
{
  "shortcut": "Ctrl+Shift+R",
  "provider": "openai",
  "language": null,
  "openai_api_key": "sk-...",
  "mistral_api_key": "..."
}
```

**API Key Sources (Priority Order):**
1. Settings file (`~/.config/whis/settings.json`)
2. Environment variables (`OPENAI_API_KEY`, `MISTRAL_API_KEY`)

**Validation Rules:**
- OpenAI keys must start with `sk-`
- Mistral keys must be ≥20 characters
- Language codes must be valid ISO-639-1 (2 lowercase letters)

---

## Requirements

### System Requirements

| Component | Requirement |
|-----------|-------------|
| **Operating System** | Linux (X11/Wayland), macOS, Windows (partial) |
| **Audio** | Working microphone |
| **Network** | Internet connection for API calls |
| **FFmpeg** | Required for CLI/Desktop (not for mobile) |

### External Dependencies

| Dependency | Purpose | Installation |
|------------|---------|--------------|
| **FFmpeg** | Audio encoding | `apt install ffmpeg` / `brew install ffmpeg` |
| **OpenAI API Key** | Transcription | [platform.openai.com/api-keys](https://platform.openai.com/api-keys) |
| **Mistral API Key** | Transcription (alternative) | [console.mistral.ai/api-keys](https://console.mistral.ai/api-keys) |

---

## Architecture Highlights

### Design Philosophy
- **Separation of Concerns** — Core logic in `whis-core`, UI-specific code in respective crates
- **Feature Flags** — Desktop vs. mobile encoder selection via Cargo features
- **Async-First** — Tokio runtime with proper concurrency control
- **Security-Conscious** — Restricted file permissions, no data retention

### Crate Dependencies (Key Libraries)

| Library | Purpose |
|---------|---------|
| `cpal` | Cross-platform audio capture |
| `hound` | WAV file I/O |
| `mp3lame-encoder` | Embedded MP3 encoding |
| `reqwest` | HTTP client for API calls |
| `tokio` | Async runtime |
| `arboard` | System clipboard |
| `rdev` | Raw device events (Linux hotkeys) |
| `global-hotkey` | Cross-platform shortcuts |
| `tauri` | Desktop/mobile framework |
| `clap` | CLI argument parsing |
| `serde` | Configuration serialization |

---

## Unique Selling Points

### Versus Local Transcription (Whisper.cpp, faster-whisper)
- **No GPU Required** — Works on any machine with internet
- **Lower Resource Usage** — No 1-4GB model downloads
- **Always Latest Model** — API uses production Whisper, not quantized versions
- **Trade-off:** Requires network, has per-minute cost

### Versus Other Voice-to-Text Apps
- **Multi-Provider** — First to offer both OpenAI and Mistral in one tool
- **AI Workflow Focus** — Designed specifically for prompt input
- **CLI-First** — Power users get a native terminal experience
- **Open Source** — Full transparency, no telemetry, self-hostable future
- **Linux-Native** — First-class X11 and Wayland support

### Versus Browser-Based Solutions
- **System-Wide** — Works in any application via global hotkeys
- **No Browser Required** — Native performance, lower memory
- **Persistent Service** — Background daemon, always ready

---

## Current Limitations

| Limitation | Status |
|------------|--------|
| **Windows Global Hotkeys** | In development |
| **Offline Mode** | Not planned (API-focused design) |
| **Real-time Transcription** | Not supported (batch processing) |
| **Custom Model Support** | API-only, no local model loading |
| **Mobile App** | Early development stage |

---

## Competitive Landscape (Research Summary)

*Based on December 2024 competitive analysis*

### Direct Competitors

| Tool | Funding/Model | Key Innovation | Weakness for Whis to Exploit |
|------|---------------|----------------|------------------------------|
| **Wispr Flow** | $81M VC | Context-aware formatting, "backtrack" corrections, IDE integrations | Cloud-only, closed-source, ~$15/mo |
| **Superwhisper** | Bootstrapped | Modes system, BYOK for any provider, clipboard+screen context | macOS-only (Windows beta), GUI-only |
| **HyperWhisper** | One-time purchase | 30+ models, 8 providers, partially open-source | macOS-only, no Linux or Windows |
| **MacWhisper** | $69 lifetime | System dictation, Obsidian integration, Nvidia Parakeet models | macOS-only |

### Key Patterns from Competitors

1. **Post-processing is the differentiator** — Wispr Flow's value isn't transcription, it's the AI cleanup
2. **Modes/templates system** — Superwhisper and Notta let users define context-specific prompts
3. **Backtrack/correction handling** — "actually 3 PM" should output "3 PM", not the full ramble
4. **Brain dump features** — AudioPen, Voicenotes target "rambling thoughts → structured output"
5. **IDE integrations** — Wispr Flow's Cursor/Windsurf extensions for voice-to-code workflows

### Feature Gap Analysis

| Feature | Wispr | Superwhisper | HyperWhisper | Whis Today | Whis Opportunity |
|---------|-------|--------------|--------------|------------|------------------|
| Open Source | ❌ | ❌ | Partial | ✅ | Keep |
| CLI Interface | ❌ | ❌ | ❌ | ✅ | Unique advantage |
| Linux Native | ❌ | ❌ | ❌ | ✅ | Unique advantage |
| Windows Support | ✅ | Beta | ❌ | Partial | In development |
| LLM Post-Process | ✅ | ✅ | ✅ | ❌ | **Priority 1** |
| Custom Modes | Limited | ✅ | ✅ | ❌ | Priority 2 |
| Backtrack Detection | ✅ | Via prompt | Via prompt | ❌ | Priority 3 |
| BYOK | ❌ | ✅ | ✅ | ✅ | Already have |
| Local Transcription | ❌ | ✅ | ✅ | ❌ | Future |

### The Opportunity

Whis can become the **"developer's Wispr Flow"** by adding LLM post-processing while maintaining:
- Open-source transparency
- CLI-first ergonomics
- Linux-native support
- No subscription model

The existing multi-provider architecture (OpenAI + Mistral) means the foundation is already in place.

---

## Implementation Plan: LLM Post-Processing Pipeline

*Approved feature based on competitive analysis*

### Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Behavior** | Opt-in | Non-breaking change, user must explicitly enable |
| **Providers** | OpenAI + Mistral | Reuse existing API keys, no new credentials |
| **Default** | Disabled | Raw transcript unless `--polish` flag or config |
| **Models** | gpt-5-nano / mistral-small-latest | Cost-efficient, fast enough for short transcripts |

### Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `crates/whis-core/src/polish.rs` | **CREATE** | Core polishing module |
| `crates/whis-core/src/lib.rs` | MODIFY | Export new module |
| `crates/whis-core/src/settings.rs` | MODIFY | Add polishing settings |
| `crates/whis-cli/src/commands/config.rs` | MODIFY | Add config flags |
| `crates/whis-cli/src/commands/record_once.rs` | MODIFY | Integrate polishing |
| `crates/whis-cli/src/service.rs` | MODIFY | Integrate polishing |
| `crates/whis-cli/src/args.rs` | MODIFY | Add `--polish` flag |
| `crates/whis-desktop/src/commands.rs` | MODIFY | Desktop app integration |

---

### Step 1: Create Polishing Module

**File:** `crates/whis-core/src/polish.rs`

```rust
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

const OPENAI_CHAT_URL: &str = "https://api.openai.com/v1/chat/completions";
const MISTRAL_CHAT_URL: &str = "https://api.mistral.ai/v1/chat/completions";
const DEFAULT_TIMEOUT_SECS: u64 = 60;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Polisher {
    #[default]
    None,
    OpenAI,
    Mistral,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct Message {
    content: String,
}

pub async fn polish(
    text: &str,
    polisher: &Polisher,
    api_key: &str,
    prompt: &str,
) -> Result<String> {
    match polisher {
        Polisher::None => Ok(text.to_string()),
        Polisher::OpenAI => polish_openai(text, api_key, prompt).await,
        Polisher::Mistral => polish_mistral(text, api_key, prompt).await,
    }
}

async fn polish_openai(text: &str, api_key: &str, system_prompt: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let response = client
        .post(OPENAI_CHAT_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({
            "model": "gpt-5-nano-2025-08-07",
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": text}
            ]
        }))
        .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT_SECS))
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow!("OpenAI polish failed: {}", error_text));
    }

    let chat_response: ChatResponse = response.json().await?;
    chat_response
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .ok_or_else(|| anyhow!("No response from OpenAI"))
}

async fn polish_mistral(text: &str, api_key: &str, system_prompt: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let response = client
        .post(MISTRAL_CHAT_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({
            "model": "mistral-small-latest",
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": text}
            ]
        }))
        .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT_SECS))
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow!("Mistral polish failed: {}", error_text));
    }

    let chat_response: ChatResponse = response.json().await?;
    chat_response
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .ok_or_else(|| anyhow!("No response from Mistral"))
}
```

---

### Step 2: Extend Settings

**File:** `crates/whis-core/src/settings.rs`

Add to `Settings` struct:
```rust
pub struct Settings {
    // ... existing fields ...
    pub polisher: Polisher,
    pub polish_prompt: Option<String>,
}
```

Add default prompt constant:
```rust
pub const DEFAULT_POLISH_PROMPT: &str =
    "Clean up this voice transcript. Fix grammar and punctuation. \
     Remove filler words (um, uh, like, you know). \
     If the speaker corrects themselves, keep only the correction. \
     Preserve technical terms and proper nouns. Output only the cleaned text.";
```

---

### Step 3: Add CLI Arguments

**File:** `crates/whis-cli/src/args.rs`

Add to main command:
```rust
#[arg(long, help = "Polish transcript with LLM (cleanup grammar, filler words)")]
pub polish: bool,
```

**File:** `crates/whis-cli/src/commands/config.rs`

Add config options:
```rust
#[arg(long, help = "Set polisher (none, openai, mistral)")]
pub polisher: Option<String>,

#[arg(long, help = "Set polish prompt")]
pub polish_prompt: Option<String>,
```

---

### Step 4: Integrate into Record Flow

**File:** `crates/whis-cli/src/commands/record_once.rs`

After transcription, before clipboard:
```rust
let final_text = if args.polish || settings.polisher != Polisher::None {
    let polisher = if args.polish && settings.polisher == Polisher::None {
        // Use same provider as transcription if not configured
        match settings.provider {
            TranscriptionProvider::OpenAI => Polisher::OpenAI,
            TranscriptionProvider::Mistral => Polisher::Mistral,
        }
    } else {
        settings.polisher.clone()
    };

    let api_key = match polisher {
        Polisher::OpenAI => settings.get_openai_api_key()?,
        Polisher::Mistral => settings.get_mistral_api_key()?,
        Polisher::None => unreachable!(),
    };

    let prompt = settings.polish_prompt
        .as_deref()
        .unwrap_or(DEFAULT_POLISH_PROMPT);

    println!("Polishing...");
    polish(&transcription, &polisher, &api_key, prompt).await?
} else {
    transcription
};

copy_to_clipboard(&final_text)?;
```

---

### Step 5: Integrate into Service

**File:** `crates/whis-cli/src/service.rs`

Same pattern in `stop_and_transcribe()` method.

---

### CLI Usage After Implementation

```bash
# Configure polishing (one-time)
whis config --polisher openai
whis config --polish-prompt "Clean transcript for AI prompts"

# Use with flag (override for single recording)
whis --polish                    # Uses configured polisher

# Or configure to always polish
whis config --polisher openai    # Now all recordings are polished

# Disable polishing
whis config --polisher none

# View config
whis config --show
```

---

### Estimated Implementation Scope

| Component | Lines | Complexity |
|-----------|-------|------------|
| `polish.rs` | ~100 | Low (follows transcribe.rs pattern) |
| Settings changes | ~30 | Low |
| CLI args | ~20 | Low |
| record_once.rs | ~25 | Low |
| service.rs | ~25 | Low |
| config.rs | ~40 | Low |
| Desktop integration | ~30 | Medium |
| **Total** | **~270** | **Low-Medium** |

---

### Testing Checklist

- [ ] `whis` without flag → raw transcript (no change)
- [ ] `whis --polish` → polished transcript
- [ ] `whis config --polisher openai` persists
- [ ] `whis config --polisher none` disables
- [ ] Custom prompt works
- [ ] API errors handled gracefully (fallback to raw transcript?)
- [ ] Service mode respects settings
- [ ] Desktop app respects settings

---

## Getting Started (For Future Development)

### Quick Start

1. **Read this document** — Understand why we're building polishing
2. **Review the codebase patterns** — Look at `transcribe.rs` for the provider pattern
3. **Start with `polish.rs`** — Create the new module first
4. **Test incrementally** — Get CLI working before desktop integration

### Key Files to Understand

```
crates/whis-core/src/
├── transcribe.rs    # Pattern to follow (provider enum, async API calls)
├── settings.rs      # Where to add new config fields
├── config.rs        # TranscriptionProvider enum (duplicate pattern)
└── lib.rs           # Add new module export

crates/whis-cli/src/
├── args.rs          # Add --polish flag
├── commands/
│   ├── config.rs    # Add config options
│   └── record_once.rs  # Main integration point
└── service.rs       # Background service integration
```

### Design Principles to Maintain

1. **Opt-in by default** — Never break existing behavior
2. **Reuse existing API keys** — No new credential types
3. **Follow the provider pattern** — Enum + match + async functions
4. **Keep it simple** — ~270 lines total, not a rewrite

### What Success Looks Like

```bash
# Before (still works)
whis                    # Raw transcript → clipboard

# After (new capability)
whis --polish           # Clean transcript → clipboard
whis config --polisher openai  # Enable by default
```

---

## Future Roadmap (After Post-Processing)

### Priority 2: Modes System (High Impact, Medium Effort)

**Why:** Superwhisper's killer feature. Different contexts need different polishing.

**Implementation:**

1. **Modes directory:** `~/.config/whis/modes/`
   ```
   modes/
   ├── default.json      # No polishing
   ├── prompt.json       # Clean for AI prompts
   ├── email.json        # Formal, professional
   ├── code.json         # Preserve technical terms
   └── braindump.json    # Summarize & extract action items
   ```

2. **Mode schema:**
   ```json
   {
     "name": "prompt",
     "description": "Clean transcript for AI prompts",
     "polisher": "openai",
     "system_prompt": "Clean up this voice transcript for use as an AI prompt. Remove filler words, fix grammar, but preserve the user's intent and technical terminology.",
     "model": "gpt-5-nano-2025-08-07"
   }
   ```

3. **CLI usage:**
   ```bash
   whis --mode prompt          # Use prompt mode
   whis --mode braindump       # Summarize rambling thoughts
   whis config --default-mode prompt
   whis modes list             # Show available modes
   whis modes create           # Interactive mode creation
   ```

4. **Desktop integration:**
   - Mode selector in settings UI
   - Quick mode switch in tray menu

---

### Priority 3: Backtrack/Correction Handling (Medium Impact, Low Effort)

**Why:** Wispr Flow's "actually" detection is a UX win. Natural speech includes self-corrections.

**Implementation:**

Add to polish prompts:
```
When the speaker corrects themselves (e.g., "2 PM... actually 3 PM" or
"the function foo... I mean bar"), output only the corrected version.
```

This requires no new architecture—just a well-crafted default prompt in the modes system.

---

### Priority 4: Context Flag for Quick Mode Selection (Low Effort, High UX)

**Why:** Wispr Flow's context-awareness without the complexity.

```bash
whis --context slack      # Casual, emoji-friendly
whis --context email      # Formal, professional
whis --context code       # Preserve camelCase, technical terms
whis --context claude     # Optimized for Claude prompts
```

Maps to pre-defined modes, simpler than full mode system for quick use.

---

### Priority 5: Output Format Options (Medium Impact, Low Effort)

**Why:** MacWhisper and Brain Dump apps emphasize Obsidian integration.

```bash
whis --output markdown    # Add YAML frontmatter
whis --output json        # Structured output
whis --output plain       # Current behavior (default)
```

Markdown output example:
```markdown
---
date: 2024-01-15T10:30:00
duration: 45s
mode: braindump
---

# Voice Note

[Transcript content here]

## Action Items
- Item extracted by LLM
```

---

### Priority 6: Local Transcription Option (High Effort, Strategic)

**Why:** Privacy-conscious users currently underserved. whisper.cpp integration.

**Considerations:**
- Adds significant binary size (model files)
- Requires careful feature flagging
- Consider as separate `whis-local` crate
- Could use `whisper-rs` crate (Rust bindings to whisper.cpp)

**Recommendation:** Defer until core polishing features are solid. The current API-based approach is a valid differentiator for users who prioritize simplicity over privacy.

---

### Implementation Order Recommendation

| Phase | Feature | Effort | Impact | Dependencies |
|-------|---------|--------|--------|--------------|
| **1** | LLM Post-Processing | Medium | High | None |
| **2** | Context Flags | Low | High | Phase 1 |
| **3** | Modes System | Medium | High | Phase 1 |
| **4** | Backtrack Detection | Low | Medium | Phase 1 |
| **5** | Output Formats | Low | Medium | None |
| **6** | Local Transcription | High | Medium | None |

---

### Quick Win: Immediate Value with Minimal Code

If you want to ship something fast, **Priority 1 alone** (basic polishing with a single configurable prompt) delivers 80% of the value:

```bash
# User configures once
whis config --polisher openai
whis config --polish-prompt "Clean this transcript: fix grammar, remove filler words (um, uh, like), handle self-corrections, output clean text ready to paste into Claude."

# Then every recording is automatically cleaned
whis  # Records, transcribes, polishes, copies clean text
```

This single feature would differentiate Whis from every other CLI transcription tool and match the core value prop of $81M-funded Wispr Flow.

---

### Architecture Insight

The existing codebase is well-positioned for this evolution:

```
Current:  Audio → Transcribe → Clipboard
Future:   Audio → Transcribe → [Post-Process] → [Format] → Clipboard
                                    ↑              ↑
                              Mode/Context     Output format
```

The multi-provider pattern (`TranscriptionProvider` enum) directly extends to `Polisher` enum. No architectural changes needed—just additive modules.

---

*Document updated with feature roadmap. Current as of version 0.5.9.*

---

## Appendix: Fact-Check Notes

*Corrections applied December 2024*

### Pricing Corrections

| Item | Original Claim | Corrected | Source |
|------|----------------|-----------|--------|
| Mistral Voxtral | ~$0.006/min | **~$0.001/min** | [Mistral Official](https://mistral.ai/news/voxtral) |
| MacWhisper | $35 lifetime | **$69 lifetime** | Gumroad |
| HyperWhisper | $39 one-time | One-time purchase (price varies) | Website |

### Platform Corrections

| Tool | Original Claim | Corrected |
|------|----------------|-----------|
| HyperWhisper | "Windows/Mac only" | **macOS only** (no Windows version exists) |
| Superwhisper | "macOS-only" | **macOS primary, Windows in beta** |

### Claim Refinements

**Original:** "Whis is the only open-source, CLI-first, Linux-native voice transcription tool."

**Corrected:** "Whis is unique in combining: open-source Rust implementation, API-based transcription (no local GPU), multi-provider support (OpenAI + Mistral), and first-class Linux support with global hotkeys."

**Rationale:** Other open-source CLI tools exist (OpenAI Whisper CLI, faster-whisper, nerd-dictation), but none combine all of Whis's differentiators.

### Verified Correct (No Changes Needed)

- OpenAI Whisper pricing: ~$0.006/min ✅
- Wispr Flow funding: $81M ✅
- All technical constants (chunk size, overlap, timeouts) verified against codebase ✅
- API endpoints verified ✅
- Model names verified ✅
