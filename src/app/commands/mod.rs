use serde::{Deserialize, Serialize};

mod graph;
mod system;

use crate::audio::RenderMode;
use crate::core::utils::note::Note;
pub use graph::*;
pub use system::*;

/// Commands that produce an `AudioMessage` for the render thread.
///
/// Every variant goes through `into_audio_message()` — no exceptions.
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
    SetRenderMode(RenderMode),
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

impl AudioCommand {
    /// Validate and translate into an `AudioMessage` in one step.
    ///
    /// Returns `Err` if the command has invalid parameters, `Ok(AudioMessage)` otherwise.
    pub fn into_audio_message(
        self,
    ) -> Result<crate::audio::AudioMessage, crate::audio::CommandError> {
        use crate::audio::{AudioMessage, CommandError, InstrumentAudioMessage};

        match self {
            AudioCommand::NoteStart {
                instrument_idx,
                note,
                velocity,
            } => {
                if !(0.0..=1.0).contains(&velocity) {
                    return Err(CommandError::InvalidVolume(velocity));
                }
                Ok(AudioMessage::Instrument(
                    InstrumentAudioMessage::NoteStart {
                        instrument_idx,
                        note,
                        velocity,
                    },
                ))
            }
            AudioCommand::NoteStop {
                instrument_idx,
                note,
            } => Ok(AudioMessage::Instrument(InstrumentAudioMessage::NoteStop {
                instrument_idx,
                note,
            })),
            AudioCommand::SetRenderMode(mode) => Ok(AudioMessage::SetRenderMode(mode)),
            AudioCommand::Shutdown => Ok(AudioMessage::Shutdown),
        }
    }
}

impl AppCommand {
    /// Validate an app command against the current app state.
    pub fn validate(&self) -> Result<(), crate::audio::CommandError> {
        match self {
            AppCommand::System(_) => Ok(()),
        }
    }
}
