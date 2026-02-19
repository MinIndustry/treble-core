use crate::core::graph::{Entry, Filter};
use crate::core::Block;
use rustic_derive::FilterMetaData;
use std::fmt;

use super::{HighPassFilter, LowPassFilter};

#[derive(FilterMetaData, Debug, Clone, Default)]
/// Bandpass filter using a high-pass and low-pass filter
pub struct BandPass {
    #[filter_parameter(range, 0.0, 20000.0, 1000.0)]
    pub low: f32,
    #[filter_parameter(range, 0.0, 20000.0, 1000.0)]
    pub high: f32,
    #[filter_parameter(range, 0.0, 192000.0, 44100.0)]
    pub sample_rate: f32,
    pub filters: (HighPassFilter, LowPassFilter),
    #[filter_source]
    pub source: Block,
}

impl BandPass {
    pub fn new(low: f32, high: f32, sample_rate: f32) -> Self {
        Self {
            low,
            high,
            sample_rate,
            filters: (
                HighPassFilter::new(low, sample_rate),
                LowPassFilter::new(high, sample_rate),
            ),
            source: Vec::new(),
        }
    }
}

impl fmt::Display for BandPass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bandpass filter ({} - {})", self.low, self.high)
    }
}

impl Entry for BandPass {
    fn push(&mut self, block: Block, _port: usize) {
        self.source = block;
    }
}

impl Filter for BandPass {
    fn transform(&mut self) -> Vec<Block> {
        self.filters.0.push(self.source.clone(), 0);
        let hp_output = self.filters.0.transform();
        self.filters
            .1
            .push(hp_output.into_iter().next().unwrap_or_default(), 0);
        self.filters.1.transform()
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
