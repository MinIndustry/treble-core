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
}

#[derive(Debug, Clone)]
pub enum GraphAudioMessage {
    SetParameter {
        node_index: usize,
        param_name: String,
        value: f32,
    },
    StartSource { source_index: usize },
    StopSource { source_index: usize },
    Swap(System), // Use `System` as new audio graph
    Clear,
}
