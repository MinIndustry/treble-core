use std::fmt;

#[cfg(feature = "meta")]
use rustic_derive::FilterMetaData;

use crate::core::graph::{Entry, Filter};

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "meta", derive(FilterMetaData))]
/// Low-pass filter using a first-order IIR filter
pub struct LowPassFilter {
    // add filter_source proc macro if meta feature is enabled
    #[cfg_attr(feature = "meta", filter_source)]
    sources: [f32; 1],
    #[cfg_attr(feature = "meta", filter_parameter(range, 0.0, 20000.0, 1000.0))]
    cutoff_frequency: f32,
    previous_output: f32,
}

impl LowPassFilter {
    pub fn new(cutoff_frequency: f32) -> Self {
        Self {
            sources: [0.0],
            cutoff_frequency,
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
        let alpha = self.cutoff_frequency / (self.cutoff_frequency + 1.0);
        let output = alpha * input + (1.0 - alpha) * self.previous_output;
        self.previous_output = output;
        vec![output]
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
