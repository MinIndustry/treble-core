use serde::{Deserialize, Serialize};

use super::app::App;

mod graph;
mod live;
mod system;

use crate::audio::RenderMode;
pub use graph::*;
pub use live::*;
pub use system::*;

/// Commands that produce an `AudioMessage` for the render thread.
///
/// Every variant goes through `into_audio_message()` — no exceptions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioCommand {
    NoteStart { note: u8, row: u8, velocity: f32 },
    NoteStop { note: u8, row: u8 },
    SetRenderMode(RenderMode),
    Shutdown,
}

/// Commands that mutate `App` state only (no audio thread interaction).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppCommand {
    System(SystemCommand),
    Live(LiveCommand),
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
        app: &App,
    ) -> Result<crate::audio::AudioMessage, crate::audio::CommandError> {
        use crate::audio::{AudioMessage, CommandError, InstrumentAudioMessage};

        match self {
            AudioCommand::NoteStart {
                note,
                row,
                velocity,
            } => {
                if row >= 2 {
                    return Err(CommandError::RowOutOfBounds(row));
                }
                if !(0.0..=1.0).contains(&velocity) {
                    return Err(CommandError::InvalidVolume(velocity));
                }
                let note = app.rows[row as usize].get_note(note);
                let instrument_idx = app.rows[row as usize].instrument;
                Ok(AudioMessage::Instrument(
                    InstrumentAudioMessage::NoteStart {
                        instrument_idx,
                        note,
                        velocity,
                    },
                ))
            }
            AudioCommand::NoteStop { note, row } => {
                if row >= 2 {
                    return Err(CommandError::RowOutOfBounds(row));
                }
                let note = app.rows[row as usize].get_note(note);
                let instrument_idx = app.rows[row as usize].instrument;
                Ok(AudioMessage::Instrument(InstrumentAudioMessage::NoteStop {
                    instrument_idx,
                    note,
                }))
            }
            AudioCommand::SetRenderMode(mode) => Ok(AudioMessage::SetRenderMode(mode)),
            AudioCommand::Shutdown => Ok(AudioMessage::Shutdown),
        }
    }
}

impl AppCommand {
    /// Validate an app command against the current app state.
    pub fn validate(&self, app: &App) -> Result<(), crate::audio::CommandError> {
        match self {
            AppCommand::Live(cmd) => cmd.validate(app),
            AppCommand::System(_) => Ok(()),
        }
    }
}
