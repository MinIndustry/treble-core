use crate::core::graph::{Entry, Filter};
use crate::core::{Block, CHANNELS};
use rustic_derive::FilterMetaData;
use std::fmt;

#[derive(FilterMetaData, Clone, Debug, Default)]
/// High-pass filter using a first-order IIR filter
pub struct HighPassFilter {
    #[filter_source]
    source: Block,
    #[filter_parameter(range, 0.0, 20000.0, 1000.0)]
    cutoff_frequency: f32,
    #[filter_parameter(range, 0.0, 192000.0, 44100.0)]
    sample_rate: f32,
    previous_output: [f32; CHANNELS],
    previous_input: [f32; CHANNELS],
}

impl HighPassFilter {
    pub fn new(cutoff_frequency: f32, sample_rate: f32) -> Self {
        Self {
            source: Vec::new(),
            cutoff_frequency,
            sample_rate,
            previous_output: [0.0; CHANNELS],
            previous_input: [0.0; CHANNELS],
        }
    }
}

impl Entry for HighPassFilter {
    fn push(&mut self, block: Block, _port: usize) {
        self.source = block;
    }
}

impl fmt::Display for HighPassFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "High Pass Filter - {}Hz", self.cutoff_frequency)
    }
}

impl Filter for HighPassFilter {
    fn transform(&mut self) -> Vec<Block> {
        let rc = 1.0 / (2.0 * std::f32::consts::PI * self.cutoff_frequency);
        let dt = 1.0 / self.sample_rate;
        let alpha = rc / (rc + dt);

        let output: Block = self.source
            .iter()
            .map(|frame| {
                std::array::from_fn(|ch| {
                    let y =
                        alpha * (self.previous_output[ch] + frame[ch] - self.previous_input[ch]);
                    self.previous_output[ch] = y;
                    self.previous_input[ch] = frame[ch];
                    y
                })
            })
            .collect();

        vec![output]
    }

    fn postponable(&self) -> bool {
        false
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
