use serde::{Deserialize, Serialize};

use super::notes::Note;
use super::score::TimeSignature;

#[derive(Serialize, Deserialize, Default)]
pub enum ChordModifier {
    #[default]
    None,
    Arpeggio,
    ArpeggioInverted,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Chord {
    notes: Vec<Note>,
    modifier: ChordModifier,
}

impl Chord {
    pub fn length(&self) -> f32 {
        4.0
    }
}

/// A measure contains a given amount of notes
#[derive(Serialize, Deserialize)]
pub struct Measure {
    size: usize, // Number of Crotchet notes
    notes: Vec<Chord>,
}

impl Measure {
    /// Creates a new measure with no notes.
    pub fn new(signature: &TimeSignature) -> Self {
        Self {
            size: signature.0,
            notes: Vec::from_iter((0..signature.0).map(|_| Chord::default())),
        }
    }

    /// Checks if the measure is full
    pub fn _is_full(&self) -> bool {
        true
        // self.notes.iter().map(|note| 1.0 / note.)
    }
}
