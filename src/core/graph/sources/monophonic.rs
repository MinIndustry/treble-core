use crate::{
    Note,
    core::{
        Block,
        audio::{mono_to_frame, silent_block},
        generator::prelude::MultiToneGenerator,
    },
};

use crate::core::graph::Source;

#[derive(Debug, Clone, Default)]
/// Strategies for replacing or not a playing note in the monophonic generator.
pub enum MonophonicAllocationStrategy {
    #[default]
    Replace,
    Drop,
    // TODO: Add a conditional replacement based on current output power.
}

#[derive(Debug, Clone)]
/// A monophonic source for the graph system.
pub struct MonophonicSource {
    generator: MultiToneGenerator,
    replacement_strategy: MonophonicAllocationStrategy,
    sample_rate: f32,
    /// When false, `start_note()` triggers the generator without updating its frequency.
    /// Set to false for percussive instruments with fixed tuning (kick, snare, etc.).
    track_pitch: bool,
    active: bool,
    released: bool,
    current_note: Option<Note>,
}

impl MonophonicSource {
    pub fn new(
        generator: MultiToneGenerator,
        sample_rate: f32,
        replacement_strategy: MonophonicAllocationStrategy,
    ) -> Self {
        Self {
            generator,
            replacement_strategy,
            sample_rate,
            track_pitch: true,
            active: false,
            released: false,
            current_note: None,
        }
    }

    pub fn new_percussive(
        generator: MultiToneGenerator,
        sample_rate: f32,
        replacement_strategy: MonophonicAllocationStrategy,
    ) -> Self {
        Self {
            generator,
            replacement_strategy,
            sample_rate,
            track_pitch: false,
            active: false,
            released: false,
            current_note: None,
        }
    }

    fn should_replace(&self) -> bool {
        // TODO: Update with power based replacement strategy
        matches!(
            self.replacement_strategy,
            MonophonicAllocationStrategy::Replace
        )
    }
}

impl From<MultiToneGenerator> for MonophonicSource {
    fn from(generator: MultiToneGenerator) -> Self {
        Self::new(generator, 44100.0, MonophonicAllocationStrategy::Replace)
    }
}

impl Source for MonophonicSource {
    fn pull(&mut self, block_size: usize) -> Block {
        if !self.active {
            return silent_block(block_size);
        }

        let dt = 1.0 / self.sample_rate; // Delta time
        let samples = self.generator.tick_block(block_size, dt);
        if self.released && self.generator.completed() {
            self.active = false;
            self.released = false;
        }
        samples.into_iter().map(mono_to_frame).collect()
    }

    fn start(&mut self) {
        if self.active {
            match self.replacement_strategy {
                MonophonicAllocationStrategy::Replace => {
                    self.generator.start();
                    self.active = true;
                    self.released = false;
                }
                MonophonicAllocationStrategy::Drop => {}
            }
        } else {
            self.generator.start();
            self.active = true;
            self.released = false;
        }
    }

    fn stop(&mut self) {
        self.generator.stop();
        self.released = true;
    }

    fn kill(&mut self) {
        self.generator.stop();
        self.active = false;
        self.released = false;
    }

    fn start_note(&mut self, note: crate::Note, _velocity: f32) {
        if self.should_replace() {
            self.current_note = Some(note);
            if self.track_pitch {
                self.generator.set_base_frequency(note.frequency());
            }
            self.start();
        }
    }

    fn stop_note(&mut self, note: crate::Note) {
        if !self.track_pitch {
            self.stop();
            return;
        }
        if let Some(current_note) = self.current_note
            && current_note == note
        {
            self.stop();
        }
    }

    fn is_active(&self) -> bool {
        self.active
    }
}
