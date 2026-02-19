use crate::core::graph::{Entry, Filter};
use crate::core::{Block, CHANNELS};
use rustic_derive::FilterMetaData;
use std::{collections::VecDeque, fmt};

/// A moving average filter implementation.
/// Based only on the previous samples
#[derive(FilterMetaData, Debug, Clone)]
pub struct MovingAverage {
    #[filter_parameter(val, 3, 0)]
    size: usize,
    /// Per-channel circular buffers
    buffers: [VecDeque<f32>; CHANNELS],
    #[filter_source]
    source: Block,
}

impl Default for MovingAverage {
    fn default() -> Self {
        Self::new(3)
    }
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
            buffers: std::array::from_fn(|_| VecDeque::from(vec![0.0f32; size])),
            source: Vec::new(),
        }
    }
}

impl Entry for MovingAverage {
    fn push(&mut self, block: Block, _port: usize) {
        self.source = block;
    }
}

impl Filter for MovingAverage {
    fn transform(&mut self) -> Vec<Block> {
        let size = self.size.max(1) as f32;
        let output: Block = self.source
            .iter()
            .map(|frame| {
                std::array::from_fn(|ch| {
                    self.buffers[ch].push_back(frame[ch]);
                    if self.buffers[ch].len() > self.size {
                        self.buffers[ch].pop_front();
                    }
                    self.buffers[ch].iter().sum::<f32>() / size
                })
            })
            .collect();
        vec![output]
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
