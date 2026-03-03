use super::prelude::VoiceMode;
use crate::core::graph::System;

// TODO (Phase 4): derive Serialize/Deserialize once System supports it
pub struct CustomInstrument {
    pub name: String,
    pub voice_mode: VoiceMode,
    pub system: System,
}
