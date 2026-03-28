use std::fmt;
use std::sync::Arc;

use rustic_derive::FilterMetaData;

use super::{HighPassFilter, LowPassFilter};
use crate::core::Block;
use crate::core::graph::{Entry, Filter};

#[derive(FilterMetaData, Debug, Clone, Default)]
/// Bandpass filter using a high-pass and low-pass filter
pub struct BandPass {
    #[filter_parameter(range, 1.0, 20000.0, 1000.0)]
    pub low: f32,
    #[filter_parameter(range, 1.0, 20000.0, 1000.0)]
    pub high: f32,
    pub sample_rate: f32,
    pub filters: (HighPassFilter, LowPassFilter),
    #[filter_source]
    pub source: Arc<Block>,
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
            source: Arc::new(Vec::new()),
        }
    }
}

impl fmt::Display for BandPass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bandpass filter ({} - {})", self.low, self.high)
    }
}

impl Entry for BandPass {
    fn push(&mut self, block: Arc<Block>, _port: usize) {
        self.source = block;
    }
}

impl Filter for BandPass {
    fn transform(&mut self) -> Vec<Block> {
        self.filters.0.push(Arc::clone(&self.source), 0);
        let hp_output = self.filters.0.transform();
        self.filters.1.push(
            Arc::new(hp_output.into_iter().next().unwrap_or_default()),
            0,
        );
        self.filters.1.transform()
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
