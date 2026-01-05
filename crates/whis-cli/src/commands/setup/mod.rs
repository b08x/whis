//! Setup wizard for different usage modes
//!
//! Provides a streamlined setup experience for:
//! - Cloud users (API key setup)
//! - Local users (on-device transcription)

mod cloud;
mod interactive;
mod local;
mod post_processing;
mod provider_helpers;

use anyhow::Result;

use crate::args::SetupMode;
use crate::ui::prompt_choice_with_default;

pub fn run(mode: Option<SetupMode>) -> Result<()> {
    match mode {
        None => setup_wizard(),
        Some(SetupMode::Cloud) => cloud::setup_cloud(),
        Some(SetupMode::Local) => local::setup_local(),
        Some(SetupMode::PostProcessing) => post_processing::setup_post_processing(),
    }
}

/// Unified setup wizard - guides user through all configuration
fn setup_wizard() -> Result<()> {
    interactive::header("whis setup");

    // Step 1: Transcription - Cloud or Local?
    println!("How do you want to transcribe?");
    println!("  1. Cloud (OpenAI, Groq, etc.) - Fast, requires API key");
    println!("  2. Local (on-device) - Private, no internet needed");
    println!();

    let is_cloud = match prompt_choice_with_default("Select", 1, 2, Some(1))? {
        1 => {
            println!();
            cloud::setup_transcription_cloud()?;
            true
        }
        2 => {
            println!();
            local::setup_transcription_local()?;
            false
        }
        _ => unreachable!(),
    };

    // Step 2: Post-processing (independent of transcription choice)
    post_processing::setup_post_processing_step(is_cloud)?;

    println!();
    interactive::success("Configuration saved! Run 'whis' to record and transcribe.");

    Ok(())
}
