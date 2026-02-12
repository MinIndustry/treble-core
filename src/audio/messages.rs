//! Audio control messages sent from command thread to audio thread

use crate::core::Note;
use crate::core::graph::System;

use super::RenderMode;

/// Messages sent from command thread to audio render thread
#[derive(Debug, Clone)]
pub enum AudioMessage {
    Instrument(InstrumentAudioMessage),
    Graph(GraphAudioMessage),

    SetRenderMode(RenderMode),

    // System control
    SetMasterVolume { volume: f32 },
    SetSampleRate { rate: u32 },

    // Lifecycle
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum InstrumentAudioMessage {
    // Note control
    NoteStart {
        instrument_idx: usize,
        note: Note,
        velocity: f32,
    },
    NoteStop {
        instrument_idx: usize,
        note: Note,
    },

    // Instrument control
    SetOctave {
        row: usize,
        octave: u8,
    },
}

#[derive(Debug, Clone)]
pub enum GraphAudioMessage {
    SetParameter {
        node_index: usize,
        param_name: String,
        value: f32,
    },
    Swap(System), // Use `System` as new audio graph
    Clear,
}
