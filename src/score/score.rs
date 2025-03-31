use log::info;
use serde::{Deserialize, Serialize};

use super::notes::Note;
use super::staff::Staff;
use crate::instruments::Instrument;

/// A simple time signature denoted with its numerator and denominator.
/// ```rust
/// use rustic::prelude::score::TimeSignature;
///
/// // A simple 4/4 time signature
/// let time_signature = TimeSignature(4, 4);
/// ```
#[derive(Serialize, Deserialize)]
pub struct TimeSignature(pub usize, pub usize);

/// A music score. Has a defined time signature, tempo,
/// staves associated with their instruments, name, ...
#[derive(Serialize, Deserialize)]
pub struct Score {
    pub name: String,             // Name of the score
    pub signature: TimeSignature, // Time signature of the score
    pub tempo: usize,             // Tempo in bpm
    pub staves: Vec<Staff>,       // Staves of the score, contains the instruments
    #[serde(skip)]
    pub instruments: Vec<Box<dyn Instrument>>,
}

impl Score {
    pub fn new<S: AsRef<str>>(
        name: S,
        signature: TimeSignature,
        tempo: usize,
        staves: usize,   // Number of instruments
        measures: usize, // Number of measures in the score
    ) -> Self {
        let staves = Vec::from_iter(
            (0..staves).map(|_| Staff::default().with_measures(measures, &signature)),
        );
        Self {
            name: name.as_ref().to_string(),
            signature,
            tempo,
            staves,
            instruments: Vec::new(),
        }
    }

    /// Adds an instrument to the score and returns its index.
    /// The instrument's index is also added to the corresponding staff.
    /// If the number of staves is less than the number of instruments, a new staff is created.
    /// ```rust
    /// use rustic::prelude::score::{Score, TimeSignature};
    /// use rustic::instruments::prelude::{HiHat, Kick, Snare};
    ///
    /// let mut score = Score::new("Test", TimeSignature(4, 4), 120, 1, 20);
    /// let kick_index = score.add_instrument(Box::new(Kick::new()));
    /// ```
    pub fn add_instrument(&mut self, instrument: Box<dyn Instrument>) -> usize {
        let new_index = self.instruments.len();
        if self.staves.len() < new_index {
            info!("Adding a new staff to the score");
            self.staves.push(Staff::default());
        }

        self.instruments.push(instrument);
        let instrument_index = self.instruments.len() - 1;
        self.staves[new_index].set_instrument(instrument_index);
        instrument_index
    }

    /// Adds a note to the score at the specified position.
    pub fn add_note(&mut self, instrument: usize, note: Note) -> Result<(), String> {
        if instrument >= self.instruments.len() {
            return Err("Instrument index out of bounds".to_string());
        }

        let staff = if let Some(staff) = self
            .staves
            .iter_mut()
            .find(|staff| staff.get_instrument() == instrument)
        {
            staff
        } else {
            return Err("No staff found for the instrument".to_string());
        };

        staff.add_note(note)
    }
}
