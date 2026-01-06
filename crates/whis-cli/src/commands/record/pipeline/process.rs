//! Post-processing pipeline phase

use anyhow::{Result, anyhow};
use whis_core::{
    DEFAULT_POST_PROCESSING_PROMPT, PostProcessor, Preset, Settings, ollama, post_process,
};

use super::super::types::{ProcessedResult, TranscriptionResult};
use crate::app;

/// Post-processing configuration
pub struct ProcessingConfig {
    pub enabled: bool,
    pub preset: Option<Preset>,
}

/// Execute post-processing phase
pub async fn process(
    transcription: TranscriptionResult,
    config: &ProcessingConfig,
    quiet: bool,
) -> Result<ProcessedResult> {
    let mut text = transcription.text;

    // If post-processing is enabled OR a preset is provided, apply LLM processing
    if config.enabled || config.preset.is_some() {
        let settings = Settings::load();
        let (processor, api_key, model, prompt) =
            resolve_post_processor(&config.preset, &settings)?;

        if !quiet {
            app::print_status(" Post-processing...", None);
        }

        text = post_process(&text, &processor, &api_key, &prompt, model.as_deref()).await?;
    }

    Ok(ProcessedResult { text })
}

/// Resolve post-processing configuration from settings and preset
fn resolve_post_processor(
    preset: &Option<Preset>,
    settings: &Settings,
) -> Result<(PostProcessor, String, Option<String>, String)> {
    // Determine which post-processor to use
    let processor = if let Some(p) = preset {
        if let Some(post_processor_str) = &p.post_processor {
            post_processor_str
                .parse()
                .unwrap_or(settings.post_processing.processor.clone())
        } else {
            settings.post_processing.processor.clone()
        }
    } else {
        settings.post_processing.processor.clone()
    };

    // Determine prompt: preset > settings > default
    let prompt = if let Some(p) = preset {
        p.prompt.clone()
    } else {
        settings
            .post_processing
            .prompt
            .clone()
            .unwrap_or_else(|| DEFAULT_POST_PROCESSING_PROMPT.to_string())
    };

    // Get API key/URL and model based on processor type
    match processor {
        PostProcessor::Ollama => {
            // Start Ollama if not running
            let ollama_url = settings
                .services
                .ollama
                .url()
                .ok_or_else(|| anyhow!("Ollama URL not configured"))?;
            ollama::ensure_ollama_running(&ollama_url)?;

            // Model priority: preset > settings
            let model = if let Some(p) = preset {
                p.model.clone()
            } else {
                settings.services.ollama.model()
            };

            if model.is_none() {
                return Err(anyhow!("Ollama model not configured"));
            }

            Ok((PostProcessor::Ollama, ollama_url, model, prompt))
        }
        PostProcessor::OpenAI => {
            let api_key = settings
                .post_processing
                .api_key(&settings.transcription.api_keys)
                .ok_or_else(|| {
                    anyhow!(
                        "OpenAI API key not configured. Set it with: whis config --openai-api-key <key>"
                    )
                })?;

            // Model from preset if available
            let model = preset.as_ref().and_then(|p| p.model.clone());

            Ok((PostProcessor::OpenAI, api_key, model, prompt))
        }
        PostProcessor::Mistral => {
            let api_key = settings
                .post_processing
                .api_key(&settings.transcription.api_keys)
                .ok_or_else(|| {
                    anyhow!(
                        "Mistral API key not configured. Set it with: whis config --mistral-api-key <key>"
                    )
                })?;

            // Model from preset if available
            let model = preset.as_ref().and_then(|p| p.model.clone());

            Ok((PostProcessor::Mistral, api_key, model, prompt))
        }
        PostProcessor::None => Err(anyhow!("Post-processing not configured. Run: whis setup")),
    }
}
