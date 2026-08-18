use std::collections::VecDeque;
use std::fmt;
use std::sync::Arc;

use treble_derive::FilterMetaData;

use crate::core::graph::{Entry, Filter};
use crate::core::{Block, CHANNELS, Frame};

/// Compact Schroeder-style ambience built from four parallel feedback combs.
#[derive(FilterMetaData, Clone)]
pub struct ReverbFilter {
    #[filter_source]
    source: Arc<Block>,
    #[filter_parameter(range, 0.0, 1.0, 0.3)]
    amount: f32,
    #[filter_parameter(range, 1.0, 192000.0, 44100.0)]
    sample_rate: f32,
    buffers: Vec<VecDeque<Frame>>,
    configured_for: (f32, f32),
}

impl ReverbFilter {
    const DELAYS: [f32; 4] = [0.0297, 0.0371, 0.0411, 0.0437];

    pub fn new(sample_rate: f32, amount: f32) -> Self {
        let mut filter = Self {
            source: Arc::new(Vec::new()),
            amount,
            sample_rate,
            buffers: Vec::new(),
            configured_for: (-1.0, -1.0),
        };
        filter.ensure_buffers();
        filter
    }

    fn ensure_buffers(&mut self) {
        let configuration = (self.amount, self.sample_rate);
        if configuration == self.configured_for {
            return;
        }
        let room_scale = 0.75 + self.amount.clamp(0.0, 1.0) * 0.75;
        self.buffers = Self::DELAYS
            .iter()
            .map(|delay| {
                let frames = (delay * room_scale * self.sample_rate).round().max(1.0) as usize;
                VecDeque::from(vec![[0.0; CHANNELS]; frames])
            })
            .collect();
        self.configured_for = configuration;
    }
}

impl Default for ReverbFilter {
    fn default() -> Self {
        Self::new(44_100.0, 0.3)
    }
}

impl Entry for ReverbFilter {
    fn push(&mut self, block: Arc<Block>, _port: usize) {
        self.source = block;
    }
}

impl fmt::Display for ReverbFilter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Reverb Filter - {:.0}%", self.amount * 100.0)
    }
}

impl fmt::Debug for ReverbFilter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReverbFilter")
            .field("amount", &self.amount)
            .field("sample_rate", &self.sample_rate)
            .finish()
    }
}

impl Filter for ReverbFilter {
    fn transform(&mut self) -> Vec<Block> {
        self.ensure_buffers();
        let amount = self.amount.clamp(0.0, 1.0);
        let feedback = 0.55 + amount * 0.35;
        let mut output = Vec::with_capacity(self.source.len());
        for input in self.source.iter() {
            let mut wet = [0.0; CHANNELS];
            for buffer in &mut self.buffers {
                let delayed = buffer.pop_front().unwrap_or([0.0; CHANNELS]);
                buffer.push_back(std::array::from_fn(|channel| {
                    input[channel] + delayed[channel] * feedback
                }));
                for channel in 0..CHANNELS {
                    wet[channel] += delayed[channel] / Self::DELAYS.len() as f32;
                }
            }
            output.push(std::array::from_fn(|channel| {
                input[channel] * (1.0 - amount) + wet[channel] * amount
            }));
        }
        self.source = Arc::new(Vec::new());
        vec![output]
    }

    fn postponable(&self) -> bool {
        true
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
