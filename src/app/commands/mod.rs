use serde::{Deserialize, Serialize};

use super::app::App;

mod effect;
mod graph;
mod live;
mod looping;
mod mix;
mod performance;
mod settings;
mod system;

pub use effect::*;
pub use graph::*;
pub use live::*;
pub use looping::*;
pub use mix::*;
pub use performance::*;
pub use settings::*;
pub use system::*;

/// Commands that always produce an AudioMessage for the render thread
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioCommand {
    NoteStart {
        note: u8,
        row: u8,
        velocity: f32,
    },
    NoteStop {
        note: u8,
        row: u8,
    },
    GraphSetParameter {
        node_id: u64,
        param_name: String,
        value: f32,
    },
    GraphPlay,
    GraphPause,
    GraphStop,
    Shutdown,
}

/// Commands that only mutate App/UI state (no audio thread interaction)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppCommand {
    System(SystemCommand),
    Live(LiveCommand),
    Loop(LoopCommand),
    Performance(PerformanceCommand),
    Mix(MixCommand),
    Effect(EffectCommand),
    Graph(GraphCommand),
    Settings(SettingsCommand),
}

/// Top-level command enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    Audio(AudioCommand),
    App(AppCommand),
}

impl AudioCommand {
    /// Validate an audio command against the current app state
    pub fn validate(&self, _app: &App) -> Result<(), crate::audio::CommandError> {
        use crate::audio::CommandError;

        match self {
            AudioCommand::NoteStart { row, velocity, .. } => {
                if *row >= 2 {
                    return Err(CommandError::RowOutOfBounds(*row));
                }
                if *velocity < 0.0 || *velocity > 1.0 {
                    return Err(CommandError::InvalidVolume(*velocity));
                }
                Ok(())
            }
            AudioCommand::NoteStop { row, .. } => {
                if *row >= 2 {
                    return Err(CommandError::RowOutOfBounds(*row));
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Convert this audio command into an AudioMessage for the render thread.
    /// This is infallible — every AudioCommand always produces an AudioMessage.
    pub fn into_audio_message(self, app: &App) -> crate::audio::AudioMessage {
        use crate::audio::{AudioMessage, InstrumentAudioMessage};

        match self {
            AudioCommand::NoteStart {
                note,
                row,
                velocity,
            } => {
                let note = app.rows[row as usize].get_note(note);
                let instrument_idx = app.rows[row as usize].instrument;
                AudioMessage::Instrument(InstrumentAudioMessage::NoteStart {
                    instrument_idx,
                    note,
                    velocity,
                })
            }
            AudioCommand::NoteStop { note, row } => {
                let note = app.rows[row as usize].get_note(note);
                let instrument_idx = app.rows[row as usize].instrument;
                AudioMessage::Instrument(InstrumentAudioMessage::NoteStop {
                    instrument_idx,
                    note,
                })
            }
            AudioCommand::GraphSetParameter { .. }
            | AudioCommand::GraphPlay
            | AudioCommand::GraphPause
            | AudioCommand::GraphStop => {
                // These are handled specially in the command thread via handle_audio_graph_command
                // This arm should not be reached directly — panic to catch misuse
                unreachable!("Graph audio commands are handled in the command thread dispatch")
            }
            AudioCommand::Shutdown => AudioMessage::Shutdown,
        }
    }
}

impl AppCommand {
    /// Validate an app command against the current app state
    pub fn validate(&self, app: &App) -> Result<(), crate::audio::CommandError> {
        match self {
            AppCommand::Live(cmd) => cmd.validate(app),
            AppCommand::System(_) => Ok(()),
            AppCommand::Loop(_) => Ok(()),
            AppCommand::Performance(_) => Ok(()),
            AppCommand::Mix(_) => Ok(()),
            AppCommand::Effect(_) => Ok(()),
            AppCommand::Graph(_) => Ok(()),
            AppCommand::Settings(_) => Ok(()),
        }
    }
}
