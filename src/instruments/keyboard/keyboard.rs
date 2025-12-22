use std::collections::HashMap;

use crate::core::envelope::prelude::ADSREnvelope;
use crate::core::generator::prelude::{
    tones::{SineWave, WhiteNoise},
    MultiSourceGenerator,
};
use crate::core::utils::tones::{NOTES, TONES_FREQ};
use crate::instruments::voices::{PolyVoiceAllocator, PolyphonicVoice};
use crate::instruments::Instrument;
use crate::KeyboardGenerator;
use crate::Note;

#[derive(Debug)]
pub struct Keyboard {
    generators: Vec<(Box<dyn KeyboardGenerator>, bool)>,
    allocator: PolyVoiceAllocator,
    note_indices: HashMap<Note, usize>,
    output: f32,
}

impl PolyphonicVoice for Keyboard {
    fn with_voices(mut self, voices: usize) -> Self {
        self.generators = Vec::with_capacity(voices);
        self
    }

    fn with_allocator(mut self, allocator: PolyVoiceAllocator) -> Self {
        self.allocator = allocator;
        self
    }
}

impl Keyboard {
    pub fn new(voices: usize, voice_allocator: PolyVoiceAllocator, envelope: ADSREnvelope) -> Self {
        let generators = std::iter::repeat_with(|| {
            let generator = MultiSourceGenerator::new(
                Box::from(envelope.clone()),
                Box::from(SineWave::new(0.0, 1.0)),
            )
            .add_generator(Box::from(WhiteNoise::new(1.0)), 0.01);
            let generator: Box<dyn KeyboardGenerator> = Box::from(generator);
            (generator, false)
        })
        .take(voices)
        .collect();

        Self {
            generators,
            allocator: voice_allocator,
            note_indices: HashMap::new(),
            output: 0.0,
        }
    }

    pub fn get_current_notes(&self) -> Vec<Note> {
        self.note_indices.keys().cloned().collect()
    }
}

impl Instrument for Keyboard {
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
            .sum::<f32>()
            / VOICES as f32
    }
}
