use std::collections::HashMap;

use crate::core::envelope::prelude::ADSREnvelope;
use crate::core::generator::prelude::{SimpleGenerator, SineWave};
use crate::core::generator::EnvelopedGenerator;
use crate::core::generator::FrequencyTransition;
use crate::core::tones::TONES_FREQ;
use crate::instruments::Instrument;
use crate::KeyboardGenerator;
use crate::Note;

#[derive(Debug)]
pub struct Keyboard<const VOICES: usize> {
    generators: [(Box<dyn KeyboardGenerator>, bool); VOICES],
    note_indices: HashMap<Note, usize>,
    output: f32,
}

impl<const VOICES: usize> Keyboard<VOICES> {
    pub fn new() -> Self {
        let envelope = ADSREnvelope::new()
            .with_attack(0.3, 1.0, Some((0.3, 0.0)))
            .with_decay(0.2, 0.6, Some((0.3, 0.6)))
            .with_release(5.0, 0.0, Some((0.0, 0.0)));
        let generators: [(Box<dyn KeyboardGenerator>, bool); VOICES] = std::array::from_fn(|_| {
            let generator = SimpleGenerator::new(
                Box::from(envelope.clone()),
                Box::from(SineWave::new(0.0, 1.0)),
            );
            let generator: Box<dyn KeyboardGenerator> = Box::from(generator);
            (generator, false)
        });

        Self {
            generators,
            note_indices: HashMap::new(),
            output: 0.0,
        }
    }

    pub fn get_current_notes(&self) -> Vec<Note> {
        self.note_indices.keys().cloned().collect()
    }
}

impl<const VOICES: usize> Instrument for Keyboard<VOICES> {
    /// Starts playing the given note
    fn start_note(&mut self, note: Note, _velocity: f32) {
        let generator_position = self
            .generators
            .iter()
            .position(|(_, is_playing)| !is_playing);
        if let Some(position) = generator_position {
            // If there is a free generator, we use it
            self.generators[position]
                .0
                .set_frequency(TONES_FREQ[note.0 as usize][note.1 as usize]);
            self.generators[position].0.start();
            self.generators[position].1 = true;
            self.note_indices.insert(note, position);
        } else {
            // If there is no free generator, we do not play the note
            return;
        }
    }

    /// Stops playing the given note
    fn stop_note(&mut self, note: Note) {
        if let Some(position) = self.note_indices.get(&note) {
            self.generators[*position].0.stop()
        }
    }

    /// Returns the current output of the instrument
    fn get_output(&mut self) -> f32 {
        self.output
    }

    /// Advances the instrument by one tick
    fn tick(&mut self) {
        // Stop completed generators
        for i in 0..self.generators.len() {
            if self.generators[i].1 {
                self.generators[i].1 = !self.generators[i].0.completed();
            }
        }

        self.output = self
            .generators
            .iter_mut()
            .map(|(generator, is_playing)| {
                if *is_playing {
                    let val = generator.tick(1.0 / 44100.0);
                    val
                } else {
                    0.0
                }
            })
            .sum()
    }
}
