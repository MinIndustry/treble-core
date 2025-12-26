use rand;
use core::f32;
use std::ops::Rem;

use crate::core::{envelope::Envelope, generator::{SingleToneGenerator, prelude::FrequencyRelation}};
use super::Generator;

#[derive(Debug)]
pub struct ToneGenerator {
    waveform: super::prelude::Waveform,
    frequency_relation: Option<super::prelude::FrequencyRelation>,
    pitch_envelope: Option<Box<dyn Envelope>>,
    amplitude_envelope: Box<dyn Envelope>,
    phase: f32,
    normalized_time: f32,
    is_stopped: bool,
    current_frequency: f32,
}

impl Generator for ToneGenerator {
    fn start(&mut self) {
        self.normalized_time = 0.0;
    }

    fn stop(&mut self) {
        self.normalized_time = 1.0;
    }

    fn completed(&self) -> bool {
        self.normalized_time >= 1.0
    }

    fn tick(&mut self, time_elapsed: f32) -> f32 {
        // 2 * pi * [[ (t - t0) / T ]]
        self.phase += 2.0 * f32::consts::PI * time_elapsed * self.current_frequency;
        println!("Angle: {} - f: {}", f32::to_degrees(self.phase).rem(360.0), self.current_frequency);

        0.0
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
            normalized_time: 0.0,
            is_stopped: true,
            current_frequency: frequency,
        }
    }
}