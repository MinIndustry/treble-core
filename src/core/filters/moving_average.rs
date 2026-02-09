use std::fmt;

#[cfg(feature = "meta")]
use rustic_derive::FilterMetaData;

use crate::core::graph::{AudioGraphElement, Entry, Filter};

/// A moving average filter implementation.
/// Based only on the previous samples
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "meta", derive(FilterMetaData))]
pub struct MovingAverage {
    index: usize,
    #[cfg_attr(feature = "meta", filter_parameter(int, 3, 0))]
    size: usize,
    buffer: Vec<f32>,
    #[cfg_attr(feature = "meta", filter_source)]
    source: f32,
}

impl fmt::Display for MovingAverage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Moving average filter 4 samples")
    }
}

impl MovingAverage {
    pub fn new(size: usize) -> Self {
        Self {
            index: 0,
            size,
            buffer: vec![0.0; size],
            source: 0.0,
        }
    }
}

impl AudioGraphElement for MovingAverage {
    fn get_name(&self) -> &str {
        "Moving Average Filter"
    }

    fn get_index(&self) -> usize {
        self.index
    }

    fn set_index(&mut self, index: usize) {
        self.index = index;
    }
}

impl Entry for MovingAverage {
    fn push(&mut self, value: f32, _port: usize) {
        self.source = value;
    }
}

impl Filter for MovingAverage {
    fn transform(&mut self) -> Vec<f32> {
        let output =
            (self.buffer.iter().fold(0.0, |p, e| p + e) + self.source) / (self.size + 1) as f32;
        for i in (self.size - 1)..0 {
            self.buffer[i] = self.buffer[i - 1];
        }
        self.buffer[0] = self.source;

        vec![output]
    }

    fn postponable(&self) -> bool {
        false
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
