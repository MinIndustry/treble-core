use serde::{Deserialize, Serialize};

mod graph;
mod system;

use crate::core::utils::note::Note;
pub use graph::*;
pub use system::*;

/// Commands that produce an `AudioMessage` for the render thread.
///
/// App translates these into internal `AudioMessage`s (routing by source index)
/// before forwarding to the render thread — the render thread never sees these.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioCommand {
    NoteStart {
        instrument_idx: usize,
        note: Note,
        velocity: f32,
    },
    NoteStop {
        instrument_idx: usize,
        note: Note,
    },
    Shutdown,
}

/// Commands that mutate `AppState` only (no audio thread interaction).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppCommand {
    System(SystemCommand),
}

/// Top-level command envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    Audio(AudioCommand),
    App(AppCommand),
    Graph(GraphCommand),
}

impl AppCommand {
    /// Validate an app command against the current app state.
    pub fn validate(&self) -> Result<(), crate::audio::CommandError> {
        match self {
            AppCommand::System(_) => Ok(()),
        }
    }
}
