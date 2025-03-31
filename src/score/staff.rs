use serde::{Deserialize, Serialize};

use super::{measure::Measure, notes::Note, score::TimeSignature};

/// A staff is an instrument line.

/// A staff is a single line in a music score. It has an associated instrument,
/// contains multiple measures with their notes
#[derive(Serialize, Deserialize, Default)]
pub struct Staff {
    instrument: usize, // Instrument index in the intrument map
    measures: Vec<Measure>,
}

impl Staff {
    pub fn with_measures(mut self, measures: usize, signature: &TimeSignature) -> Self {
        self.measures = Vec::from_iter((0..measures).map(|_| Measure::new(signature)));
        self
    }

    pub fn set_instrument(&mut self, instrument: usize) {
        self.instrument = instrument;
    }

    pub fn get_instrument(&self) -> usize {
        self.instrument
    }

    pub fn add_note(&mut self, _note: Note) -> Result<(), String> {
        let _last_measure = self.measures.last_mut().ok_or("No measures in the staff")?;
        //last_measure.add_note(note)
        Ok(())
    }
}
