use serde::{Deserialize, Serialize};

use super::{
    measure::{Chord, Measure},
    notes::Note,
    score::TimeSignature,
};

/// A staff is a single line in a music score. It has an associated instrument,
/// contains multiple measures with their notes
#[derive(Serialize, Deserialize, Default, Clone)]
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
                self.measures.last_mut().unwrap()
            };

        let position = measure.current_index();
        measure.add_note(position, note)
    }

    pub fn get_orderer_chords(&self) -> Vec<Chord> {
        self.measures
            .iter()
            .flat_map(|m| m.get_orderer_chords())
            .collect()
    }

    /// Calculates the total duration of this staff in ticks
    pub fn total_duration(&self) -> usize {
        let chords = self.get_orderer_chords();
        chords.iter().map(|chord| chord.duration()).sum()
    }
}
