use super::{ToneGenerator, VariableFrequency, VariableToneGenerator};

#[derive(Debug)]
/// A generator that does nothing. Can be used as a placeholder
/// or for testing purposes.
pub struct NullGenerator;

impl NullGenerator {
    pub fn new() -> Self {
        Self
    }
}

impl ToneGenerator for NullGenerator {
    fn tick(&mut self, _elapsed_time: f32) -> f32 {
        0.0
    }
}

impl VariableFrequency for NullGenerator {
    fn change_frequency(&mut self, _: f32, _: super::FrequencyTransition) {}
}

impl VariableToneGenerator for NullGenerator {}
