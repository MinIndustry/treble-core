use serde::{Deserialize, Serialize};

use crate::prelude::App;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LiveCommand {
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
