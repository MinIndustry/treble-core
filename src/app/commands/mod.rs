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

/// Top-level command enum that groups related commands into sub-enums
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    System(SystemCommand),
    Live(LiveCommand),
    Loop(LoopCommand),
    Performance(PerformanceCommand),
    Mix(MixCommand),
    Effect(EffectCommand),
    Graph(GraphCommand),
    Settings(SettingsCommand),
}

impl Command {
    /// Validate a command against the current app state
    pub fn validate(&self, app: &App) -> Result<(), crate::audio::CommandError> {
        match self {
            Command::Live(cmd) => cmd.validate(app),
            Command::System(_) => Ok(()),
            Command::Loop(_) => Ok(()),
            Command::Performance(_) => Ok(()),
            Command::Mix(_) => Ok(()),
            Command::Effect(_) => Ok(()),
            Command::Graph(_) => Ok(()),
            Command::Settings(_) => Ok(()),
        }
    }

    /// Translate a command to an audio message for the audio render thread
    pub fn translate_to_audio_message(&self, app: &mut App) -> Option<crate::audio::AudioMessage> {
        use crate::audio::AudioMessage;

        match self {
            Command::Live(LiveCommand::NoteStart {
                note,
                row,
                velocity,
            }) => {
                let note = app.rows[*row as usize].get_note(*note);
                let instrument_idx = app.rows[*row as usize].instrument;
                Some(AudioMessage::NoteStart {
                    instrument_idx,
                    note,
                    velocity: *velocity,
                })
            }
            Command::Live(LiveCommand::NoteStop { note, row }) => {
                let note = app.rows[*row as usize].get_note(*note);
                let instrument_idx = app.rows[*row as usize].instrument;
                Some(AudioMessage::NoteStop {
                    instrument_idx,
                    note,
                })
            }
            Command::Live(LiveCommand::OctaveUp(row))
            | Command::Live(LiveCommand::OctaveDown(row))
            | Command::Live(LiveCommand::SetOctave { row, .. }) => Some(AudioMessage::SetOctave {
                row: *row as usize,
                octave: app.rows[*row as usize].octave,
            }),
            Command::System(SystemCommand::Quit) => Some(AudioMessage::Shutdown),
            _ => None,
        }
    }
}
