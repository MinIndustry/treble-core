use std::fmt;

#[cfg(feature = "meta")]
use rustic_derive::FilterMetaData;

use crate::core::graph::{AudioGraphElement, Entry, Filter};

/// High-pass filter using a first-order IIR filter
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "meta", derive(FilterMetaData))]
pub struct HighPassFilter {
    #[cfg_attr(feature = "meta", filter_source)]
    sources: [f32; 1],
    #[cfg_attr(feature = "meta", filter_parameter(range, 0.0, 20000.0, 1000.0))]
    cutoff_frequency: f32,
    previous_output: f32,
    index: usize,
}

impl HighPassFilter {
    pub fn new(cutoff_frequency: f32) -> Self {
        Self {
            sources: [0.0],
            cutoff_frequency,
            previous_output: 0.0,
            index: 0,
        }
    }
}

impl Entry for HighPassFilter {
    fn push(&mut self, value: f32, port: usize) {
        self.sources[port] = value;
    }
}

impl fmt::Display for HighPassFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "High Pass Filter - {}Hz", self.cutoff_frequency)
    }
}

impl Filter for HighPassFilter {
    fn transform(&mut self) -> Vec<f32> {
        let input = self.sources[0];
        let alpha = 1.0 / (self.cutoff_frequency + 1.0);
        let output = alpha * input + alpha * self.previous_output;
        self.previous_output = output;
        vec![output]
    }

    fn postponable(&self) -> bool {
        false
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl AudioGraphElement for HighPassFilter {
    fn get_name(&self) -> &str {
        "High Pass Filter"
    }

    fn get_index(&self) -> usize {
        self.index
    }

    fn set_index(&mut self, index: usize) {
        self.index = index;
    }
}
