use log::trace;
use serde::{Deserialize, Serialize};

use crate::core::{
    envelope::Envelope,
    generator::{prelude::MixMode, tone::SingleToneGenerator},
};

#[derive(Default, Debug, Serialize, Deserialize)]
/// A generator that produces multiple tones. Each
/// tone can have its own frequency relation, waveform,
/// and envelopes.
pub struct MultiToneGenerator {
    base_frequency: f32,
    tone_generators: Vec<SingleToneGenerator>,
    mix_mode: super::prelude::MixMode,
    global_pitch_envelope: Option<Box<dyn Envelope>>,
    global_amplitude_envelope: Option<Box<dyn Envelope>>,
    time: f32,
    note_off: Option<f32>,
}

impl MultiToneGenerator {
    pub fn new(
        base_frequency: f32,
        mut tone_generators: Vec<super::tone::SingleToneGenerator>,
        mix_mode: super::prelude::MixMode,
        global_pitch_envelope: Option<Box<dyn Envelope>>,
        global_amplitude_envelope: Option<Box<dyn Envelope>>,
    ) -> Self {
        // Setup individual tone generators' frequencies
        tone_generators.iter_mut().for_each(|tg| {
            if !tg.has_frequency_relation() {
                log::warn!("Adding a tone generator without a frequency relation to a composite generator. The generator will not get updated");
            } else {
                tg.update_frequency(base_frequency);
            }
        });

        Self {
            base_frequency,
            tone_generators,
            mix_mode,
            global_pitch_envelope,
            global_amplitude_envelope,
            time: 0.0,
            note_off: None,
        }
    }

    pub fn start(&mut self) {
        trace!("Composite Generator starting ({}Hz)", self.base_frequency);
        self.time = 0.0;
        self.note_off = None;
        // Reset all child tone generators to ensure clean retriggering
        self.tone_generators.iter_mut().for_each(|tg| tg.start());
    }

    pub fn stop(&mut self) {
        trace!(
            "Composite Generator stopping: {} ({}Hz)",
            self.time,
            self.base_frequency
        );
        self.note_off = Some(self.time);
        self.tone_generators.iter_mut().for_each(|tg| tg.stop());
    }

    pub fn completed(&self) -> bool {
        // A composite generator is completed when all its tone generators are completed
        // or when there's a note_off and sufficient time has passed for all envelopes to finish
        if self.tone_generators.is_empty() {
            return true;
        }

        // Check if all tone generators have completed
        self.tone_generators.iter().all(|tg| tg.completed())
    }

    pub fn tick(&mut self, time_elapsed: f32) -> f32 {
        let actual_elapsed = if let Some(envelope) = &self.global_pitch_envelope {
            time_elapsed * envelope.at(self.time, self.note_off.unwrap_or(0.0))
        } else {
            time_elapsed
        };

        // Map true time elapsed for pitch bend
        self.time += time_elapsed;

        let values = self
            .tone_generators
            .iter_mut()
            .map(|g| g.tick(actual_elapsed))
            .collect::<Vec<f32>>();

        let ampl = match self.mix_mode {
            MixMode::Average => values.iter().sum::<f32>() / values.len() as f32,
            MixMode::Multiply => values.iter().fold(1.0, |a, v| a * v),
            MixMode::Max => values.iter().fold(f32::NEG_INFINITY, |a, v| a.max(*v)),
            MixMode::Sum => values.iter().sum(),
        };

        if let Some(envelope) = &self.global_amplitude_envelope {
            trace!(
                "Composite Generator ticking: t:{} e:{} a:{} ae:{} ({}Hz)",
                self.time,
                envelope.at(self.time, self.note_off.unwrap_or(0.0)),
                ampl,
                ampl * envelope.at(self.time, self.note_off.unwrap_or(0.0)),
                self.base_frequency
            );

            ampl * envelope.at(self.time, self.note_off.unwrap_or(0.0))
        } else {
            trace!(
                "Composite Generator ticking: t:{} a:{} ({}Hz)",
                self.time,
                ampl,
                self.base_frequency
            );

            ampl
        }
    }

    pub fn add_tone(&mut self, tone: super::tone::SingleToneGenerator) {
        self.tone_generators.push(tone);
    }

    pub fn set_base_frequency(&mut self, frequency: f32) {
        self.base_frequency = frequency;
        for generator in self.tone_generators.iter_mut() {
            generator.update_frequency(frequency);
        }
    }

    pub fn tone_count(&self) -> usize {
        self.tone_generators.len()
    }
}
