//! Audio control messages sent directly from App to the render thread.
//!
//! These are *internal* messages. The frontend-facing API is [`crate::app::commands::Command`],
//! which App translates into these messages before forwarding to the render thread.

use crate::core::Note;
use crate::core::graph::System;

/// Messages sent from App to the audio render thread.
#[derive(Debug, Clone)]
pub enum AudioMessage {
    /// Instrument note control — routed by source index in the compiled System.
    Instrument(InstrumentAudioMessage),
    /// Graph structural/playback control — for the visual graph editor.
    Graph(GraphAudioMessage),
    // Lifecycle
    Shutdown,
}

/// Note-on / note-off for a specific source in the compiled System.
///
/// `source_index` is resolved by App from the user-facing `instrument_idx`
/// via `AudioGraph::source_map` — the render thread never sees instrument indices.
#[derive(Debug, Clone)]
pub enum InstrumentAudioMessage {
    NoteStart {
        source_index: usize,
        note: Note,
        velocity: f32,
    },
    NoteStop {
        source_index: usize,
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
    StartSource {
        source_index: usize,
    },
    StopSource {
        source_index: usize,
    },
    /// Replace the entire running graph with a freshly compiled one.
    Swap(System),
    Clear,
}
