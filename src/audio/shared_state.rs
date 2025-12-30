//! Shared state between audio threads using atomic types

use atomic_float::AtomicF32;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64};

/// State shared between threads using lock-free atomics
pub struct SharedAudioState {
    pub shutdown: AtomicBool,
    pub buffer_underruns: AtomicU64,
    pub sample_rate: AtomicU32,
    pub master_volume: AtomicF32,
}

impl SharedAudioState {
    pub fn new() -> Self {
        Self {
            shutdown: AtomicBool::new(false),
            buffer_underruns: AtomicU64::new(0),
            sample_rate: AtomicU32::new(44100),
            master_volume: AtomicF32::new(1.0),
        }
    }
}

impl Default for SharedAudioState {
    fn default() -> Self {
        Self::new()
    }
}
