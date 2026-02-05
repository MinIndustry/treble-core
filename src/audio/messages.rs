//! Audio control messages sent from command thread to audio thread

use crate::core::Note;

/// Messages sent from command thread to audio render thread
#[derive(Debug, Clone)]
pub enum AudioMessage {
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

    // System control
    SetMasterVolume {
        volume: f32,
    },
    SetSampleRate {
        rate: u32,
    },

    // Lifecycle
    Shutdown,
}
