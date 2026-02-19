use crate::core::graph::{Entry, Filter};
use crate::core::Block;
use rustic_derive::FilterMetaData;
use std::fmt;

/// A Tremolo filter, that changes sound amplitude on a sinusoid
/// basis.
#[derive(FilterMetaData, Debug, Clone, Default)]
pub struct Tremolo {
    #[filter_source]
    source: Block,
    phase: f32,
    #[filter_parameter(range, 0.0, 20.0, 1.0)]
    pub frequency: f32,
    #[filter_parameter(range, 0.0, 1.0, 0.5)]
    pub depth: f32,
    #[filter_parameter(range, 0.0, 192000.0, 44100.0)]
    pub sample_rate: f32,
}

impl Tremolo {
    pub fn new(frequency: f32, depth: f32, sample_rate: f32) -> Self {
        Self {
            source: Vec::new(),
            phase: 0.0,
            frequency,
            depth,
            sample_rate,
        }
    }
}

impl fmt::Display for Tremolo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tremolo: {}Hz, depth: {}", self.frequency, self.depth)
    }
}

impl Entry for Tremolo {
    fn push(&mut self, block: Block, _port: usize) {
        self.source = block;
    }
}

impl Filter for Tremolo {
    fn transform(&mut self) -> Vec<Block> {
        let phase_increment =
            (2.0 * std::f32::consts::PI * self.frequency) / self.sample_rate.max(1.0);

        let output: Block = self.source
            .iter()
            .map(|frame| {
                let modulation = 1.0 - self.depth * (0.5 * (1.0 + self.phase.sin()));
                self.phase += phase_increment;
                if self.phase > 2.0 * std::f32::consts::PI {
                    self.phase -= 2.0 * std::f32::consts::PI;
                }
                std::array::from_fn(|ch| frame[ch] * modulation)
            })
            .collect();

        vec![output]
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
