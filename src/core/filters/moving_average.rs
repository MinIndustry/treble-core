use std::fmt;

use crate::core::graph::{AudioGraphElement, Entry, Filter};

/// A moving average filter implementation.
/// Based only on the previous samples
#[derive(Debug, Clone)]
pub struct MovingAverage<const SIZE: usize> {
    index: usize,
    buffer: [f32; SIZE],
    source: f32,
}

impl<const SIZE: usize> fmt::Display for MovingAverage<SIZE> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Moving average filter 4 samples")
    }
}

impl<const SIZE: usize> MovingAverage<SIZE> {
    pub fn new() -> Self {
        Self {
            index: 0,
            buffer: [0.0; SIZE],
            source: 0.0,
        }
    }
}

impl<const SIZE: usize> AudioGraphElement for MovingAverage<SIZE> {
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

impl<const SIZE: usize> Entry for MovingAverage<SIZE> {
    fn push(&mut self, value: f32, _port: usize) {
        self.source = value;
    }
}

impl<const SIZE: usize> Filter for MovingAverage<SIZE> {
    fn transform(&mut self) -> Vec<f32> {
        let output = (self.buffer.iter().fold(0.0, |p, e| p + e) + self.source) / (SIZE + 1) as f32;
        for i in (SIZE - 1)..0 {
            self.buffer[i] = self.buffer[i - 1];
        }
        self.buffer[0] = self.source;

        vec![output]
    }

    fn postponable(&self) -> bool {
        false
    }
}
