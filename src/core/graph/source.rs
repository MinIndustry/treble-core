use crate::core::generator::ToneGenerator;
use super::{AudioGraphElement, Source};

pub struct SimpleSource<T> {
    generator: T,
    sample_rate: f32,
    index: usize,
}

impl<T> SimpleSource<T> {
    pub fn new(generator: T, sample_rate: f32) -> Self {
        Self {
            generator,
            sample_rate,
            index: 0
        }
    }
}

impl<T> AudioGraphElement for SimpleSource<T> {
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

impl<T> Source for SimpleSource<T>
where
    T: ToneGenerator,
{
    fn pull(&mut self) -> f32 {
        self.generator.tick(1.0 / self.sample_rate)
    }
}

/// Creates a simple source with a given generator and frequency
pub fn simple_source<T: ToneGenerator + 'static>(generator: T) -> Box<dyn Source> {
    let source = SimpleSource {
        generator,
        sample_rate: 44100.0,
        index: 0,
    };

    Box::new(source)
}
