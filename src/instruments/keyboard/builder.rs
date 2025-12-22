use super::super::voices::PolyVoiceAllocator;
use crate::core::envelope::prelude::ADSREnvelope;

use super::Keyboard;

#[derive(Default)]
pub struct KeyboardBuilder {
    voices: usize,
    allocator: PolyVoiceAllocator,
    envelope: ADSREnvelope,
}

impl KeyboardBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_voices(mut self, voices: usize) -> Self {
        self.voices = voices;
        self
    }

    pub fn with_allocator(mut self, allocator: PolyVoiceAllocator) -> Self {
        self.allocator = allocator;
        self
    }

    pub fn with_note_envelope(mut self, note_envelope: ADSREnvelope) -> Self {
        self.envelope = note_envelope;
        self
    }

    pub fn build(self) -> Keyboard {
        Keyboard::new(self.voices, self.allocator, self.envelope)
    }
}
