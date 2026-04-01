use core::f32;
use rand::{self, Rng};
use serde::{Deserialize, Serialize};
use std::ops::Rem;

use crate::core::{envelope::Envelope, generator::prelude::*};

use super::composite_builder;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SingleToneGenerator {
    waveform: Waveform,
    frequency_relation: Option<FrequencyRelation>,
    pitch_envelope: Option<Box<dyn Envelope>>,
    amplitude_envelope: Box<dyn Envelope>,
    phase: f32,
    note_off: Option<f32>, // Time when the note turned off (stop was called)
    time: f32,
    current_frequency: f32,
    pink_b: [f32; 7], // IIR filter state for pink noise (Paul Kellet algorithm)
}

impl SingleToneGenerator {
    pub fn new(
        waveform: Waveform,
        frequency_relation: Option<FrequencyRelation>,
        pitch_envelope: Option<Box<dyn Envelope>>,
        amplitude_envelope: Box<dyn Envelope>,
        frequency: f32,
    ) -> Self {
        Self {
            waveform,
            frequency_relation,
            pitch_envelope,
            amplitude_envelope,
            phase: rand::random::<f32>().rem(360.0),
            time: 0.0,
            note_off: None,
            current_frequency: frequency,
            pink_b: [0.0; 7],
        }
    }

    pub fn start(&mut self) {
        self.time = 0.0;
        self.note_off = None;
        self.pink_b = [0.0; 7];
        // Note: We intentionally do NOT reset phase here to avoid phase discontinuities.
        // Each oscillator maintains its phase across note boundaries, which prevents clicks
        // and allows for smooth retriggering. For most musical contexts, this is desirable.
        // For a phase-reset behavior, we can consider adding a separate reset() method.
    }

    pub fn stop(&mut self) {
        self.note_off = Some(self.time);
    }

    pub fn completed(&self) -> bool {
        self.note_off
            .map(|note_off| self.amplitude_envelope.completed(self.time, note_off))
            == Some(true)
    }

    pub fn tick(&mut self, time_elapsed: f32) -> f32 {
        const TAU: f32 = 2.0 * f32::consts::PI;

        // Map true time elapsed for pitch bend
        let actual_elapsed = if let Some(envelope) = &self.pitch_envelope {
            time_elapsed * envelope.at(self.time, self.note_off.unwrap_or(0.0))
        } else {
            time_elapsed
        };
        self.time += time_elapsed;

        // 2 * pi * [[ (t - t0) / T ]]
        if self.waveform.has_frequency() {
            self.phase = (self.phase + TAU * actual_elapsed * self.current_frequency) % TAU;
        }

        let tone_value = match self.waveform {
            Waveform::Blank | Waveform::Err(_) => 1.0, // Returns 1.0 that will be mapped to the amplitude envelope
            Waveform::PinkNoise => {
                let white = rand::thread_rng().gen_range(-1.0_f32..1.0);
                self.pink_b[0] = 0.99886 * self.pink_b[0] + white * 0.0555179;
                self.pink_b[1] = 0.99332 * self.pink_b[1] + white * 0.0750759;
                self.pink_b[2] = 0.96900 * self.pink_b[2] + white * 0.153852;
                self.pink_b[3] = 0.86650 * self.pink_b[3] + white * 0.3104856;
                self.pink_b[4] = 0.55000 * self.pink_b[4] + white * 0.5329522;
                self.pink_b[5] = -0.7616 * self.pink_b[5] - white * 0.0168980;
                self.pink_b[6] = white * 0.115926;
                (self.pink_b.iter().sum::<f32>() + white * 0.5362) * 0.11
            }
            Waveform::Sawtooth => (self.phase * f32::consts::FRAC_1_PI) - 1.0,
            Waveform::Sine => f32::sin(self.phase),
            Waveform::Square => {
                if self.phase > f32::consts::PI {
                    1.0
                } else {
                    -1.0
                }
            }
            Waveform::Triangle => 1.0 - 2.0 * ((self.phase * f32::consts::FRAC_1_PI) - 1.0).abs(),
            Waveform::WhiteNoise => rand::thread_rng().gen_range(-1.0..1.0),
        };

        tone_value
            * self
                .amplitude_envelope
                .at(self.time, self.note_off.unwrap_or(0.0))
    }

    pub fn set_frequency(&mut self, frequency: f32) {
        self.current_frequency = frequency;
    }

    pub fn has_frequency_relation(&self) -> bool {
        self.frequency_relation.is_some()
    }

    pub fn get_waveform(&self) -> &Waveform {
        &self.waveform
    }

    pub fn update_frequency(&mut self, base_frequency: f32) {
        if let Some(relation) = &self.frequency_relation {
            self.current_frequency = relation.compute(base_frequency);
        }
    }
}

impl From<SingleToneGenerator> for MultiToneGenerator {
    fn from(val: SingleToneGenerator) -> Self {
        composite_builder::MultiToneGeneratorBuilder::new()
            .add_generator(val)
            .build()
    }
}
