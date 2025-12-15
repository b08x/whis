use std::fmt;
use std::str::FromStr;

/// Predefined output styles for transcript polishing
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputStyle {
    AiPrompt,
    Email,
    Notes,
}

impl OutputStyle {
    /// Get the polish prompt for this style
    pub fn prompt(&self) -> &'static str {
        match self {
            OutputStyle::AiPrompt => AI_PROMPT_PROMPT,
            OutputStyle::Email => EMAIL_PROMPT,
            OutputStyle::Notes => NOTES_PROMPT,
        }
    }

    /// Get all available style names
    pub fn all() -> &'static [&'static str] {
        &["ai-prompt", "email", "notes"]
    }
}

impl fmt::Display for OutputStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputStyle::AiPrompt => write!(f, "ai-prompt"),
            OutputStyle::Email => write!(f, "email"),
            OutputStyle::Notes => write!(f, "notes"),
        }
    }
}

impl FromStr for OutputStyle {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ai-prompt" => Ok(OutputStyle::AiPrompt),
            "email" => Ok(OutputStyle::Email),
            "notes" => Ok(OutputStyle::Notes),
            _ => Err(format!(
                "Unknown style: '{}'. Available: {}",
                s,
                OutputStyle::all().join(", ")
            )),
        }
    }
}

const AI_PROMPT_PROMPT: &str = "Clean up this voice transcript for use as an AI assistant prompt. \
Fix grammar and punctuation. Remove filler words. \
Keep it close to plain text, but use minimal markdown when it improves clarity: \
lists (ordered/unordered) for multiple items, bold for emphasis, headings only when absolutely necessary. \
Preserve the speaker's intent and technical terminology. \
Output only the cleaned text.";

const EMAIL_PROMPT: &str = "Clean up this voice transcript into an email. \
Fix grammar and punctuation. Remove filler words. \
Keep it concise. Match the sender's original tone (casual or formal). \
Do NOT add placeholder names or unnecessary formalities. \
Output only the cleaned text.";

const NOTES_PROMPT: &str = "Lightly clean up this voice transcript for personal notes. \
Fix major grammar issues and remove excessive filler words. \
Preserve the speaker's natural voice and thought structure. \
IMPORTANT: Start directly with the cleaned content. NEVER add any introduction, preamble, or meta-commentary like 'Here are the notes'. \
Output ONLY the cleaned transcript, nothing else.";

