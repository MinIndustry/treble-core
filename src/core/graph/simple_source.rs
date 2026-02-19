use super::Source;
use crate::core::audio::{Block, mono_to_frame, silent_block};
use crate::core::generator::prelude::MultiToneGenerator;

#[derive(Debug, Clone)]
pub struct SimpleSource {
    generator: MultiToneGenerator,
    sample_rate: f32,
    active: bool,
    released: bool,
}

impl SimpleSource {
    pub fn new(generator: MultiToneGenerator, sample_rate: f32) -> Self {
        Self {
            generator,
            sample_rate,
            active: false,
            released: false,
        }
    }

    pub fn boxed(self) -> Box<dyn Source> {
        Box::new(self)
    }
}

impl From<MultiToneGenerator> for SimpleSource {
    fn from(generator: MultiToneGenerator) -> Self {
        Self::new(generator, 10.0)
    }
}

impl Source for SimpleSource {
    fn pull(&mut self, block_size: usize) -> Block {
        if !self.active {
            return silent_block(block_size);
        }
        let dt = 1.0 / self.sample_rate;
        let samples = self.generator.tick_block(block_size, dt);
        if self.released && self.generator.completed() {
            self.active = false;
            self.released = false;
        }
        samples.into_iter().map(mono_to_frame).collect()
    }

    fn start(&mut self) {
        self.active = true;
        self.released = false;
        self.generator.start();
    }

    fn stop(&mut self) {
        self.generator.stop(); // Records note off + triggers release
        self.released = true;
    }

    fn kill(&mut self) {
        self.active = false;
        self.released = false;
    }

    fn is_active(&self) -> bool {
        self.active
    }
}

/// Creates a simple source with a given generator and frequency
pub fn simple_source(generator: MultiToneGenerator) -> Box<dyn Source> {
    SimpleSource::from(generator).boxed()
}
