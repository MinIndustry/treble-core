use rand::{self, Rng};
use core::f32;
use std::ops::Rem;

use crate::core::{envelope::Envelope, generator::prelude::*};
use super::Generator;

#[derive(Debug)]
pub struct ToneGenerator {
    waveform: Waveform,
    frequency_relation: Option<FrequencyRelation>,
    pitch_envelope: Option<Box<dyn Envelope>>,
    amplitude_envelope: Box<dyn Envelope>,
    phase: f32,
    note_off: Option<f32>,  // Time when the note turned off (stop was called)
    time: f32,
    current_frequency: f32,
}

impl Generator for ToneGenerator {
    fn start(&mut self) {
        self.time = 0.0;
        self.note_off = None;
        // Note: We intentionally do NOT reset phase here to avoid phase discontinuities.
        // Each oscillator maintains its phase across note boundaries, which prevents clicks
        // and allows for smooth retriggering. For most musical contexts, this is desirable.
        // For a phase-reset behavior, we can consider adding a separate reset() method.
    }

    fn stop(&mut self) {
        self.note_off = Some(self.time);
    }

    fn completed(&self) -> bool {
        self.note_off.map(|note_off| self.amplitude_envelope.completed(self.time, note_off)) == Some(true)
    }

    fn tick(&mut self, time_elapsed: f32) -> f32 {
        const TAU: f32 = 2.0 * f32::consts::PI;

        // Map true time elapsed for pitch bend
        let actual_elapsed = if let Some(envelope) = &self.pitch_envelope {
            time_elapsed * envelope.at(self.time, self.note_off.unwrap_or(0.0))
        } else {
            time_elapsed
        };
        self.time += time_elapsed;
        
        // 2 * pi * [[ (t - t0) / T ]]
        self.phase = (self.phase + TAU * actual_elapsed * self.current_frequency) % TAU;

        let tone_value = match self.waveform {
            Waveform::Blank => 1.0, // Returns 1.0 that will be mapped to the amplitude envelope
            Waveform::PinkNoise => 1.0,  // TODO impl pink noise
            Waveform::Sawtooth => (self.phase * std::f32::consts::FRAC_1_PI) - 1.0,
            Waveform::Sine => f32::sin(self.phase),
            Waveform::Square => if self.phase > f32::consts::PI { 1.0 } else { -1.0 },
            Waveform::Triangle => 1.0 - 2.0 * ((self.phase * std::f32::consts::FRAC_1_PI) - 1.0).abs(),
            Waveform::WhiteNoise => rand::thread_rng().gen_range(-1.0..1.0),
        };

        tone_value * self.amplitude_envelope.at(self.time, self.note_off.unwrap_or(0.0))
    }
}

impl SingleToneGenerator for ToneGenerator {
    fn set_frequency(&mut self, frequency: f32) {
        self.current_frequency = frequency;
    }

    fn has_frequency_relation(&self) -> bool {
        self.frequency_relation.is_some()
    }

    fn update_frequency(&mut self, base_frequency: f32) {
        if let Some(relation) = &self.frequency_relation {
            self.current_frequency = relation.compute(base_frequency);
        }
    }
}

impl ToneGenerator {
    pub fn new(
        waveform: super::prelude::Waveform,
        frequency_relation: Option<super::prelude::FrequencyRelation>,
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
        }
    }
}