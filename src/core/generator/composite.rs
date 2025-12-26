use crate::core::{envelope::Envelope, generator::{Generator, MultiToneGenerator, SingleToneGenerator}};

#[derive(Debug)]
pub struct CompositeGenerator {
    base_frequency: f32,
    tone_generators: Vec<Box<dyn super::prelude::SingleToneGenerator>>,
    mix_mode: super::prelude::MixMode,
    global_pitch_envelope: Option<Box<dyn Envelope>>,
    global_amplitude_envelope: Option<Box<dyn Envelope>>,
    normalized_time: f32,
    is_stopped: bool,
}

impl Generator for CompositeGenerator {
    fn start(&mut self) {
        self.normalized_time = 0.0;
    }

    fn stop(&mut self) {
        self.normalized_time = 1.0;
    }

    fn completed(&self) -> bool {
        self.normalized_time >= 1.0
    }

    fn tick(&mut self, _time_elapsed: f32) -> f32 {
        // TODO: Implement
        0.0
    }
}

impl MultiToneGenerator for CompositeGenerator {
    fn add_tone(&mut self, tone: super::tone::ToneGenerator) {
        self.tone_generators.push(Box::new(tone));
    }

    fn set_base_frequency(&mut self, frequency: f32) {
        self.base_frequency = frequency;
        for generator in self.tone_generators.iter_mut() {
            generator.update_frequency(frequency);
        }
    }

    fn tone_count(&self) -> usize {
        self.tone_generators.len()
    }
}

impl CompositeGenerator {
    pub fn new(
        base_frequency: f32,
        mut tone_generators: Vec<Box<dyn SingleToneGenerator>>,
        mix_mode: super::prelude::MixMode,
        global_pitch_envelope: Option<Box<dyn Envelope>>,
        global_amplitude_envelope: Option<Box<dyn Envelope>>,
    ) -> Self {
        // Setup individual tone generators' frequencies
        tone_generators.iter_mut().for_each(|tg| {
            if !tg.has_frequency_relation() {
                log::warn!("Adding a tone generator without a frequency relation to a composite generator. The generator will not get updated");
            } else {
                log::info!("Here we need to set the tone generator frequency from the base freq (using relations)");
            }
        });

        Self {
            base_frequency,
            tone_generators,
            mix_mode,
            global_pitch_envelope,
            global_amplitude_envelope,
            normalized_time: 0.0,
            is_stopped: true,
        }
    }
}