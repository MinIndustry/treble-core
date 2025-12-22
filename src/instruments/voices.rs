//! # Voice module
//! Defines the `Voice` capabilities of instruments. There are multiple possibilities
//! for how an instrument can be played.
//! ## Monophonic instruments
//! One note at a time. The project implements two algorithms for voice allocation:
//! - `DropCurrent`: When a note is started, the current note, if any, is dropped.
//! - `KeepCurrent`: When a note is started, it does not replace the current note if playing.
//!
//! ## Polyphonic instruments

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
