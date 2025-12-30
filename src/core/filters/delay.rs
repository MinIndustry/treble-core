use std::collections::VecDeque;
use std::fmt;

#[cfg(feature = "meta")]
use rustic_derive::FilterMetaData;

use crate::core::graph::{AudioGraphElement, Entry, Filter};

/// Delays its input by a fixed amount of seconds.
#[derive(Clone, Default)]
#[cfg_attr(feature = "meta", derive(FilterMetaData))]
pub struct DelayFilter {
    #[cfg_attr(feature = "meta", filter_source)]
    sources: [f32; 1],
    #[cfg_attr(feature = "meta", filter_parameter(range, 0, 20.0, 0.5))]
    delay_for: f32,
    buffer: VecDeque<f32>,
    index: usize,
}

impl DelayFilter {
    pub fn new(sample_rate: f32, delay: f32) -> Self {
        Self {
            sources: [0.0],
            delay_for: delay,
            buffer: VecDeque::from(vec![0.0; (delay * sample_rate) as usize]),
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

        self.buffer.push_back(input);
        vec![output]
    }

    fn postponable(&self) -> bool {
        true
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
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
