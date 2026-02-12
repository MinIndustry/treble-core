use super::Source;
use crate::core::generator::prelude::MultiToneGenerator;

#[derive(Debug, Clone)]
pub struct SimpleSource {
    generator: MultiToneGenerator,
    sample_rate: f32,
}

impl SimpleSource {
    pub fn new(generator: MultiToneGenerator, sample_rate: f32) -> Self {
        Self {
            generator,
            sample_rate,
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
    fn pull(&mut self) -> f32 {
        self.generator.tick(1.0 / self.sample_rate)
    }
}

/// Creates a simple source with a given generator and frequency
pub fn simple_source(generator: MultiToneGenerator) -> Box<dyn Source> {
    SimpleSource::from(generator).boxed()
}
