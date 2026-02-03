use super::{AudioGraphElement, Source};
use crate::core::generator::prelude::MultiToneGenerator;

#[derive(Debug)]
pub struct SimpleSource {
    generator: MultiToneGenerator,
    sample_rate: f32,
    index: usize,
}

impl SimpleSource {
    pub fn new(generator: MultiToneGenerator, sample_rate: f32) -> Self {
        Self {
            generator,
            sample_rate,
            index: 0,
        }
    }
}

impl AudioGraphElement for SimpleSource {
    fn get_name(&self) -> &str {
        "Simple Sine Source"
    }

    fn get_index(&self) -> usize {
        self.index
    }

    fn set_index(&mut self, index: usize) {
        self.index = index;
    }
}

impl Source for SimpleSource {
    fn pull(&mut self) -> f32 {
        self.generator.tick(1.0 / self.sample_rate)
    }
}

/// Creates a simple source with a given generator and frequency
pub fn simple_source(generator: MultiToneGenerator) -> Box<dyn Source> {
    let source = SimpleSource {
        generator,
        sample_rate: 44100.0,
        index: 0,
    };

    Box::new(source)
}
