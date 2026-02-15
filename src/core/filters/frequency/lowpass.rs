use std::fmt;

#[cfg(feature = "meta")]
use rustic_derive::FilterMetaData;

use crate::core::graph::{Entry, Filter};

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "meta", derive(FilterMetaData))]
/// Low-pass filter using a first-order IIR filter
pub struct LowPassFilter {
    #[cfg_attr(feature = "meta", filter_source)]
    sources: [f32; 1],
    #[cfg_attr(feature = "meta", filter_parameter(range, 0.0, 20000.0, 1000.0))]
    cutoff_frequency: f32,
    #[cfg_attr(feature = "meta", filter_parameter(range, 0.0, 192000.0, 44100.0))]
    sample_rate: f32,
    previous_output: f32,
}

impl LowPassFilter {
    pub fn new(cutoff_frequency: f32, sample_rate: f32) -> Self {
        Self {
            sources: [0.0],
            cutoff_frequency,
            sample_rate,
            previous_output: 0.0,
        }
    }
}

impl Entry for LowPassFilter {
    fn push(&mut self, value: f32, port: usize) {
        self.sources[port] = value;
    }
}

impl fmt::Display for LowPassFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Low Pass Filter - {}Hz", self.cutoff_frequency)
    }
}

impl Filter for LowPassFilter {
    fn transform(&mut self) -> Vec<f32> {
        let input = self.sources[0];
        let rc = 1.0 / (2.0 * std::f32::consts::PI * self.cutoff_frequency);
        let dt = 1.0 / self.sample_rate;
        let alpha = dt / (rc + dt);
        let output = alpha * input + (1.0 - alpha) * self.previous_output;
        self.previous_output = output;
        vec![output]
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
