use std::collections::HashMap;

use crate::{
    NOTES, Note,
    core::{
        Block,
        audio::{mono_to_frame, silent_block},
        generator::prelude::MultiToneGenerator,
    },
};

use super::Source;

#[derive(Debug, Clone, Default)]
/// Strategies for replacing or not a playing note in the monophonic generator.
pub enum PolyphonicAllocationStrategy {
    #[default]
    ReplaceOldest,
    ReplaceYoungest,
    ReplaceLoudest,
    ReplaceQuietest,
    ReplaceRandom,
    Drop,
}

#[derive(Debug, Clone)]
/// A monophonic source for the graph system.
pub struct PolyphonicSource {
    generator_template: MultiToneGenerator,
    // Generator, active, released
    generators: Vec<(MultiToneGenerator, bool, bool)>,
    max_voices: usize,
    replacement_strategy: PolyphonicAllocationStrategy,
    sample_rate: f32,
    // Map NOTES (with octave) to generator position in the vector and age.
    current_notes: HashMap<(NOTES, usize), (usize, usize)>,
}

impl PolyphonicSource {
    pub fn new(
        generator_template: MultiToneGenerator,
        max_voices: usize,
        sample_rate: f32,
        replacement_strategy: PolyphonicAllocationStrategy,
    ) -> Self {
        Self {
            generator_template,
            generators: Vec::new(),
            max_voices,
            replacement_strategy,
            sample_rate,
            current_notes: HashMap::new(),
        }
    }
}

impl From<MultiToneGenerator> for PolyphonicSource {
    fn from(generator: MultiToneGenerator) -> Self {
        Self::new(
            generator,
            8,
            44100.0,
            PolyphonicAllocationStrategy::default(),
        )
    }
}

impl Source for PolyphonicSource {
    fn pull(&mut self, block_size: usize) -> Block {
        let any_active = self.generators.iter().map(|(_, a, _)| *a).any(|a| a);

        if !any_active {
            return silent_block(block_size);
        }

        let dt = 1.0 / self.sample_rate; // Delta time
        let samples = self.generator_template.tick_block(block_size, dt);
        if self.released && self.generator_template.completed() {
            self.active = false;
            self.released = false;
        }
        samples.into_iter().map(mono_to_frame).collect()
    }

    fn start(&mut self) {
        if self.active {
            match self.replacement_strategy {
                PolyphonicAllocationStrategy::Replace => {
                    self.generator_template.start();
                    self.active = true;
                    self.released = false;
                }
                PolyphonicAllocationStrategy::Drop => {}
            }
        } else {
            self.generator_template.start();
            self.active = true;
            self.released = false;
        }
    }

    fn stop(&mut self) {
        self.generator_template.stop();
        self.released = true;
    }

    fn kill(&mut self) {
        self.generator_template.stop();
        self.active = false;
        self.released = false;
    }

    fn start_note(&mut self, note: crate::Note, _velocity: f32) {
        // TODO: Convert Note to frequency here.
        if self.should_replace() {
            self.current_note = Some(note);
            self.generator_template.set_base_frequency(440.0);
            self.start();
        }
    }

    fn stop_note(&mut self, note: crate::Note) {
        if let Some(current_note) = self.current_note
            && current_note == note
        {
            log::trace!("Stopping generator note");
            self.stop();
        }
    }

    fn is_active(&self) -> bool {
        self.active
    }
}
