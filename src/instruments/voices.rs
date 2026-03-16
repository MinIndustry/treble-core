//! Voice modes and allocation strategies for instruments.

use serde::{Deserialize, Serialize};

/// High-level voice mode used for instrument configuration / serialization.
#[derive(Deserialize, Serialize)]
pub enum VoiceMode {
    Monophonic,
    Polyphonic { max_voices: usize },
}

/// The monophonic voice trait. An instrument implementing this trait can play one note at a time.
pub trait MonophonicVoice {
    fn with_allocator(self, allocator: MonoVoiceAllocator) -> Self;
}

/// The polyphonic voice trait. An instrument implementing this trait can play multiple notes at a time.
pub trait PolyphonicVoice {
    fn with_voices(self, voices: usize) -> Self;
    fn with_allocator(self, allocator: PolyVoiceAllocator) -> Self;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MonoVoiceAllocator {
    #[default]
    DropCurrent,
    KeepCurrent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PolyVoiceAllocator {
    #[default]
    DropOldest,
    DropNewest,
    DropQuietest,
    DropLoudest,
    DropRandom,
}
