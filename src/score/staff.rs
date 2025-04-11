use serde::{Deserialize, Serialize};

use super::{measure::Measure, notes::Note, score::TimeSignature};

/// A staff is an instrument line.

/// A staff is a single line in a music score. It has an associated instrument,
/// contains multiple measures with their notes
#[derive(Serialize, Deserialize, Default)]
pub struct Staff {
    instrument: usize, // Instrument index in the intrument map
    measures: Vec<Measure>,
    signature: TimeSignature,
}

impl Staff {
    pub fn new(signature: &TimeSignature) -> Self {
        Self {
            signature: signature.clone(),
            ..Default::default()
        }
    }

    pub fn set_instrument(&mut self, instrument: usize) {
        self.instrument = instrument;
    }

    pub fn get_instrument(&self) -> usize {
        self.instrument
    }

    pub fn add_note(&mut self, note: Note) -> Result<(), String> {
        let measure: &mut Measure =
            if let Some(next_measure_position) = self.measures.iter().position(|m| !m.is_full()) {
                &mut self.measures[next_measure_position]
            } else {
                self.measures.push(Measure::new(&self.signature));
                &mut self.measures.last_mut().unwrap()
            };

        let position = measure.current_index();
        measure.add_note(position, note)
    }
}
