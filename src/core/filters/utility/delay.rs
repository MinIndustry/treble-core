use crate::core::graph::{Entry, Filter};
use crate::core::{Block, CHANNELS, Frame};
use rustic_derive::FilterMetaData;
use std::{collections::VecDeque, fmt};

/// Delays its input by a fixed number of seconds.
#[derive(FilterMetaData, Clone)]
pub struct DelayFilter {
    #[filter_source]
    source: Block,
    #[filter_parameter(range, 0.0, 20.0, 0.5)]
    delay_for: f32,
    buffer: VecDeque<Frame>,
    /// Stored for future use (e.g. recomputing buffer size on set_parameter)
    #[allow(dead_code)]
    sample_rate: f32,
}

impl DelayFilter {
    pub fn new(sample_rate: f32, delay: f32) -> Self {
        let n_frames = (delay * sample_rate) as usize;
        Self {
            source: Vec::new(),
            delay_for: delay,
            buffer: VecDeque::from(vec![[0.0; CHANNELS]; n_frames]),
            sample_rate,
        }
    }
}

impl Default for DelayFilter {
    fn default() -> Self {
        Self::new(44100.0, 0.5)
    }
}

impl Entry for DelayFilter {
    fn push(&mut self, block: Block, _port: usize) {
        self.source = block;
    }
}

impl fmt::Display for DelayFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Delay Filter - {}s", self.delay_for)
    }
}

impl fmt::Debug for DelayFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DelayFilter {{ delay_for: {} }}", self.delay_for)
    }
}

impl Filter for DelayFilter {
    fn transform(&mut self) -> Vec<Block> {
        let output: Block = self
            .source
            .iter()
            .map(|frame| {
                self.buffer.push_back(*frame);
                self.buffer.pop_front().unwrap_or([0.0; CHANNELS])
            })
            .collect();
        vec![output]
    }

    fn postponable(&self) -> bool {
        true
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
