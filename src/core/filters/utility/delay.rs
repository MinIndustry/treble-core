use std::sync::Arc;
use std::{collections::VecDeque, fmt};

use treble_derive::FilterMetaData;

use crate::core::graph::{Entry, Filter};
use crate::core::{Block, CHANNELS, Frame};

/// Delays its input by a fixed number of seconds.
#[derive(FilterMetaData, Clone)]
pub struct DelayFilter {
    #[filter_source]
    source: Arc<Block>,
    #[filter_parameter(range, 0.0, 20.0, 0.5)]
    delay_for: f32,
    #[filter_parameter(range, 0.0, 0.99, 0.0)]
    feedback: f32,
    #[filter_parameter(range, 0.0, 1.0, 1.0)]
    mix: f32,
    buffer: VecDeque<Frame>,
    #[filter_parameter(range, 1.0, 192000.0, 44100.0)]
    sample_rate: f32,
    configured_for: (f32, f32),
}

impl DelayFilter {
    pub fn new(sample_rate: f32, delay: f32) -> Self {
        let n_frames = (delay * sample_rate) as usize;
        Self {
            source: Arc::new(Vec::new()),
            delay_for: delay,
            feedback: 0.0,
            mix: 1.0,
            buffer: VecDeque::from(vec![[0.0; CHANNELS]; n_frames]),
            sample_rate,
            configured_for: (delay, sample_rate),
        }
    }

    fn ensure_buffer(&mut self) {
        let configuration = (self.delay_for, self.sample_rate);
        if configuration != self.configured_for {
            let frames = (self.delay_for * self.sample_rate).round().max(1.0) as usize;
            self.buffer = VecDeque::from(vec![[0.0; CHANNELS]; frames]);
            self.configured_for = configuration;
        }
    }
}

impl Default for DelayFilter {
    fn default() -> Self {
        Self::new(44100.0, 0.5)
    }
}

impl Entry for DelayFilter {
    fn push(&mut self, block: Arc<Block>, _port: usize) {
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
        write!(
            f,
            "DelayFilter {{ delay_for: {}, feedback: {}, mix: {} }}",
            self.delay_for, self.feedback, self.mix
        )
    }
}

impl Filter for DelayFilter {
    fn transform(&mut self) -> Vec<Block> {
        self.ensure_buffer();
        let mut output = Vec::with_capacity(self.source.len());
        for input in self.source.iter() {
            let delayed = self.buffer.pop_front().unwrap_or([0.0; CHANNELS]);
            self.buffer.push_back(std::array::from_fn(|channel| {
                input[channel] + delayed[channel] * self.feedback
            }));
            output.push(std::array::from_fn(|channel| {
                input[channel] * (1.0 - self.mix) + delayed[channel] * self.mix
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
