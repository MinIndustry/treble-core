//! Audio control messages sent directly from App to the render thread.
//!
//! These are *internal* messages. The frontend-facing API is [`crate::app::commands::Command`],
//! which App translates into these messages before forwarding to the render thread.

use crate::core::Note;
use crate::core::graph::{ModTarget, System};

/// Messages sent from App to the audio render thread.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
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
    SetSourceParameter {
        source_index: usize,
        param_name: String,
        value: f32,
    },
    StartSource {
        source_index: usize,
    },
    /// Graceful stop — lets the release envelope finish (may never end for infinite envelopes).
    StopSource {
        source_index: usize,
    },
    /// Hard stop — immediately silences the source regardless of envelope state.
    KillSource {
        source_index: usize,
    },
    /// Replace the entire running graph with a freshly compiled one.
    Swap(System),
    Clear,
    /// Register a live modulation wire in the running system.
    AddModulation {
        from_source: usize,
        target: ModTarget,
        param_name: String,
    },
    /// Remove a live modulation wire from the running system.
    RemoveModulation {
        from_source: usize,
        target: ModTarget,
        param_name: String,
    },
}
