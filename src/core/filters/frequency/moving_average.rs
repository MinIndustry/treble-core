use std::fmt;

#[cfg(feature = "meta")]
use rustic_derive::FilterMetaData;

use crate::core::graph::{Entry, Filter};

/// A moving average filter implementation.
/// Based only on the previous samples
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "meta", derive(FilterMetaData))]
pub struct MovingAverage {
    #[cfg_attr(feature = "meta", filter_parameter(val, 3, 0))]
    size: usize,
    buffer: Vec<f32>,
    write_pos: usize,
    #[cfg_attr(feature = "meta", filter_source)]
    source: f32,
}

impl fmt::Display for MovingAverage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Moving average filter {} samples", self.size)
    }
}

impl MovingAverage {
    pub fn new(size: usize) -> Self {
        Self {
            size,
            buffer: vec![0.0; size],
            write_pos: 0,
            source: 0.0,
        }
    }
}

impl Entry for MovingAverage {
    fn push(&mut self, value: f32, _port: usize) {
        self.source = value;
    }
}

impl Filter for MovingAverage {
    fn transform(&mut self) -> Vec<f32> {
        self.buffer[self.write_pos] = self.source;
        self.write_pos = (self.write_pos + 1) % self.size;

        let sum: f32 = self.buffer.iter().sum();
        let output = sum / self.size as f32;

        vec![output]
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
