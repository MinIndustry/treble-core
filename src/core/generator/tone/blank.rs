use crate::core::generator::{
    FrequencyTransition, ToneGenerator, VariableFrequency, VariableToneGenerator,
};

#[derive(Debug)]
/// A generator that does nothing. Can be used as a placeholder
/// or for testing purposes.
pub struct Blank;

impl Blank {
    pub fn new() -> Self {
        Self
    }
}

impl ToneGenerator for Blank {
    fn tick(&mut self, _elapsed_time: f32) -> f32 {
        0.0
    }
}

impl VariableFrequency for Blank {
    fn change_frequency(&mut self, _: f32, _: FrequencyTransition) {}
}

impl VariableToneGenerator for Blank {}
