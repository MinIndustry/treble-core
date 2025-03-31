use std::collections::VecDeque;
use std::fmt;

use log::trace;

use crate::core::graph::{AudioGraphElement, Entry, Filter};

/// Delays it input for x samples
#[derive(Clone)]
pub struct DelayFilter {
    sources: [f32; 1],
    delay_for: usize,
    buffer: VecDeque<f32>,
    index: usize,
}

impl DelayFilter {
    pub fn new(delay: usize) -> Self {
        Self {
            sources: [0.0],
            delay_for: delay,
            buffer: VecDeque::from(vec![0.0; delay]),
            index: 0,
        }
    }
}

impl Entry for DelayFilter {
    fn push(&mut self, value: f32, port: usize) {
        self.sources[port] = value;
    }
}

impl fmt::Display for DelayFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Delay Filter - {} samples", self.delay_for)
    }
}

impl fmt::Debug for DelayFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DelayFilter {{ delay_for: {}, index: {} }}",
            self.delay_for, self.index
        )
    }
}

impl Filter for DelayFilter {
    fn transform(&mut self) -> Vec<f32> {
        let input = self.sources[0];
        let output = self.buffer.pop_front().unwrap_or(0.0);

        trace!("Delay filter running {} -> {}", self.sources[0], output);

        self.buffer.push_back(input);
        vec![output]
    }

    fn postponable(&self) -> bool {
        true
    }
}

impl AudioGraphElement for DelayFilter {
    fn get_name(&self) -> &str {
        "Delay Filter"
    }

    fn get_index(&self) -> usize {
        self.index
    }

    fn set_index(&mut self, index: usize) {
        self.index = index;
    }
}
