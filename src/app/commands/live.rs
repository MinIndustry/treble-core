use serde::{Deserialize, Serialize};

use crate::prelude::App;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LiveCommand {
    NoteStart { note: u8, row: u8, velocity: f32 },
    NoteStop { note: u8, row: u8 },
    OctaveUp(u8),
    OctaveDown(u8),
    SetOctave { octave: u8, row: u8 },
    LinkOctaves,
    UnlinkOctaves,
    SelectInstrument { index: usize, row: u8 },
    NextInstrument(u8),
    PreviousInstrument(u8),
    LinkInstruments,
    UnlinkInstruments,
}

impl LiveCommand {
    pub fn validate(&self, _app: &App) -> Result<(), crate::audio::CommandError> {
        use crate::audio::CommandError;

        match self {
            LiveCommand::NoteStart { row, velocity, .. } => {
                if *row >= 2 {
                    return Err(CommandError::RowOutOfBounds(*row));
                }
                if *velocity < 0.0 || *velocity > 1.0 {
                    return Err(CommandError::InvalidVolume(*velocity));
                }
                Ok(())
            }
            LiveCommand::NoteStop { row, .. } => {
                if *row >= 2 {
                    return Err(CommandError::RowOutOfBounds(*row));
                }
                Ok(())
            }
            LiveCommand::SetOctave { octave, row } => {
                if *row >= 2 {
                    return Err(CommandError::RowOutOfBounds(*row));
                }
                if *octave > 8 {
                    return Err(CommandError::InvalidOctave(*octave));
                }
                Ok(())
            }
            LiveCommand::OctaveUp(row) | LiveCommand::OctaveDown(row) => {
                if *row >= 2 {
                    return Err(CommandError::RowOutOfBounds(*row));
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
