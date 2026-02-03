use crate::core::envelope::Envelope;

use crate::core::generator::prelude::{Waveform, MultiToneGenerator, SingleToneGenerator};

/// Builder for the `CompositeGenerator`.
pub struct MultiToneGeneratorBuilder {
    base_freq: f32,
    generators: Vec<SingleToneGenerator>,
    mix_mode: super::prelude::MixMode,
    pitch: Option<Box<dyn Envelope>>,
    amplitude: Option<Box<dyn Envelope>>,
}

impl Default for MultiToneGeneratorBuilder {
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

impl MultiToneGeneratorBuilder {
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
        generator: SingleToneGenerator,
    ) -> Self {
        if !generator.has_frequency_relation() && !matches!(generator.get_waveform(), Waveform::WhiteNoise | Waveform::PinkNoise) {
            log::warn!("Adding a tone generator without a frequency relation to a composite generator. The generator will not get updated");
        }

        self.generators.push(generator);
        self
    }

    pub fn build(self) -> MultiToneGenerator {
        MultiToneGenerator::new(
            self.base_freq,
            self.generators,
            self.mix_mode,
            self.pitch,
            self.amplitude,
        )
    }
}