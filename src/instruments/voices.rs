//! # Voice module
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub enum VoiceMode {
    Monophonic,
    Polyphonic { max_voices: usize },
}
