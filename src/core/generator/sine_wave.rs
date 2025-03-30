use super::{
    Bendable, BendableGenerator, FrequencyTransition, ToneGenerator, VariableBendableGenerator,
    VariableFrequency, VariableGenerator,
};

use crate::KeyboardGenerator;

use std::f32::consts::PI;

#[derive(Debug)]
pub struct SineWave {
    frequency: f32,
    amplitude: f32,
    timer: f32,
    pitch_ratio: f32,
}

impl SineWave {
    pub fn new(frequency: f32, amplitude: f32) -> Self {
        Self {
            frequency,
            amplitude,
            timer: 0.0,
            pitch_ratio: 1.0,
        }
    }
}

impl ToneGenerator for SineWave {
    fn tick(&mut self, elapsed_time: f32) -> f32 {
        self.timer += elapsed_time * self.pitch_ratio;
        self.amplitude * (2.0 * PI * self.frequency * self.timer).sin()
    }
}

impl VariableFrequency for SineWave {
    /// TODO: Implement frequency transition
    fn change_frequency(&mut self, frequency: f32, _transistion: FrequencyTransition) {
        self.frequency = frequency;
    }
}

impl Bendable for SineWave {
    fn set_pitch_bend(&mut self, bend: f32) {
        self.pitch_ratio = bend;
    }
}

impl BendableGenerator for SineWave {}
impl VariableGenerator for SineWave {}
impl VariableBendableGenerator for SineWave {}
impl KeyboardGenerator for SineWave {}
