use serde::{Deserialize, Serialize};

use super::notes::Note;
use super::score::TimeSignature;

/// A measure contains a given amount of notes
#[derive(Serialize, Deserialize)]
pub struct Measure {
    notes: Vec<Vec<Note>>,
}

impl Measure {
    /// Creates a new measure with no notes.
    pub fn new(signature: &TimeSignature) -> Self {
        Self {
            notes: Vec::from_iter((0..signature.0).map(|_| Vec::new())),
        }
    }

    /// Checks if the measure is full
    pub fn _is_full(&self) -> bool {
        self.notes.len() == 4
    }
}
