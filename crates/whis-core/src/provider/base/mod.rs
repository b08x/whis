//! Base implementations and shared logic for transcription providers.

mod openai_compatible;

pub(crate) use openai_compatible::{
    openai_compatible_transcribe_async, openai_compatible_transcribe_sync,
};
