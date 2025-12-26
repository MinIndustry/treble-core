use crate::core::envelope::Envelope;

use crate::core::generator::SingleToneGenerator;
use super::composite::CompositeGenerator;

/// Builder for the `CompositeGenerator`.
pub struct CompositeGeneratorBuilder {
    base_freq: f32,
    generators: Vec<Box<dyn SingleToneGenerator>>,
    mix_mode: super::prelude::MixMode,
    pitch: Option<Box<dyn Envelope>>,
    amplitude: Option<Box<dyn Envelope>>,
}

impl Default for CompositeGeneratorBuilder {
    fn default() -> Self {
        Self {
            base_freq: 440.0,
            generators: Vec::new(),
            mix_mode: super::prelude::MixMode::Sum,
            pitch: None,
            amplitude: None,
        }
    }
}

impl CompositeGeneratorBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn frequency(mut self, freq: f32) -> Self {
        self.base_freq = freq;
        self
    }

    pub fn mix_mode(mut self, mode: super::prelude::MixMode) -> Self {
        self.mix_mode = mode;
        self
    }

    pub fn pitch_envelope(
        mut self,
        envelope: Option<Box<dyn Envelope>>,
    ) -> Self {
        self.pitch = envelope;
        self
    }

    pub fn amplitude_envelope(
        mut self,
        envelope: Option<Box<dyn Envelope>>,
    ) -> Self {
        self.amplitude = envelope;
        self
    }

    pub fn add_generator(
        mut self,
        generator: Box<dyn SingleToneGenerator>,
    ) -> Self {
        if !generator.has_frequency_relation() {
            log::warn!("Adding a tone generator without a frequency relation to a composite generator. The generator will not get updated");
        }

        self.generators.push(generator);
        self
    }

    pub fn build(self) -> CompositeGenerator {
        CompositeGenerator::new(
            self.base_freq,
            self.generators,
            self.mix_mode,
            self.pitch,
            self.amplitude,
        )
    }
}