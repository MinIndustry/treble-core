use crate::core::graph::{Entry, Filter};
use crate::core::{Block, CHANNELS};
use rustic_derive::FilterMetaData;
use std::fmt;

#[derive(FilterMetaData, Clone, Debug, Default)]
/// Low-pass filter using a first-order IIR filter
pub struct LowPassFilter {
    #[filter_source]
    source: Block,
    #[filter_parameter(range, 0.0, 20000.0, 1000.0)]
    cutoff_frequency: f32,
    #[filter_parameter(range, 0.0, 192000.0, 44100.0)]
    sample_rate: f32,
    previous_output: [f32; CHANNELS],
}

impl LowPassFilter {
    pub fn new(cutoff_frequency: f32, sample_rate: f32) -> Self {
        Self {
            source: Vec::new(),
            cutoff_frequency,
            sample_rate,
            previous_output: [0.0; CHANNELS],
        }
    }
}

impl Entry for LowPassFilter {
    fn push(&mut self, block: Block, _port: usize) {
        self.source = block;
    }
}

impl fmt::Display for LowPassFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Low Pass Filter - {}Hz", self.cutoff_frequency)
    }
}

impl Filter for LowPassFilter {
    fn transform(&mut self) -> Vec<Block> {
        let rc = 1.0 / (2.0 * std::f32::consts::PI * self.cutoff_frequency);
        let dt = 1.0 / self.sample_rate;
        let alpha = dt / (rc + dt);

        let output: Block = self.source
            .iter()
            .map(|frame| {
                std::array::from_fn(|ch| {
                    let y = alpha * frame[ch] + (1.0 - alpha) * self.previous_output[ch];
                    self.previous_output[ch] = y;
                    y
                })
            })
            .collect();

        vec![output]
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
