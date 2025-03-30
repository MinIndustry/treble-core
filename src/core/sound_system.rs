use crate::tone_generator::ToneGenerator;
use crate::filter::Filter;

pub struct SoundSystem {
    pub generator: Box<dyn ToneGenerator>,
    pub filters: Vec<Box<dyn Filter>>,
}

impl SoundSystem {
    pub fn new(generator: Box<dyn ToneGenerator>) -> Self {
        Self {
            generator,
            filters: Vec::new(),
        }
    }

    pub fn add_filter(&mut self, filter: Box<dyn Filter>) {
        self.filters.push(filter);
    }

    pub fn generate_sample(&mut self, time: f32) -> f32 {
        let mut sample = self.generator.generate(time);
        for filter in &mut self.filters {
            sample = filter.apply(sample);
        }
        sample
    }
}