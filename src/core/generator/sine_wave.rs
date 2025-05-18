use super::{
    Bendable, BendableGenerator, FrequencyTransition, ToneGenerator, VariableFrequency,
    VariableToneGenerator,
};

use std::f32::consts::PI;

#[derive(Debug)]
/// A generator that produces a sine wave following the formula:
/// `y = A * sin(2 * PI * f * t)`
/// where A is the amplitude, f is the frequency, and t is the current time.
pub struct SineWave {
    frequency: f32,
    amplitude: f32,
}

impl SineWave {
    pub fn new(frequency: f32, amplitude: f32) -> Self {
        Self {
            frequency,
            amplitude,
        }
    }
}

impl ToneGenerator for SineWave {
    fn tick(&mut self, elapsed_time: f32) -> f32 {
        self.amplitude * (2.0 * PI * self.frequency * elapsed_time).sin()
    }
}

impl VariableFrequency for SineWave {
    /// TODO: Implement frequency transition
    fn change_frequency(&mut self, frequency: f32, _transistion: FrequencyTransition) {
        self.frequency = frequency;
    }
}

impl VariableToneGenerator for SineWave {}
