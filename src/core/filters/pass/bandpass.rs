use std::fmt;

use crate::core::graph::{AudioGraphElement, Entry, Filter};

use super::{HighPassFilter, LowPassFilter};

#[derive(Debug, Clone)]
pub struct BandPass {
    pub low: f32,
    pub high: f32,
    pub filters: (HighPassFilter, LowPassFilter),
    pub source: f32,
    pub index: usize,
}

impl BandPass {
    pub fn new(low: f32, high: f32) -> Self {
        Self {
            low,
            high,
            filters: (HighPassFilter::new(low), LowPassFilter::new(high)),
            source: 0.0,
            index: 0,
        }
    }
}

impl fmt::Display for BandPass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bandpass filter ({} - {})", self.low, self.high)
    }
}

impl AudioGraphElement for BandPass {
    fn get_name(&self) -> &str {
        "BandPass filter"
    }

    fn get_index(&self) -> usize {
        self.index
    }

    fn set_index(&mut self, index: usize) {
        self.index = index;
    }
}

impl Entry for BandPass {
    fn push(&mut self, value: f32, _port: usize) {
        self.source = value;
    }
}

impl Filter for BandPass {
    fn transform(&mut self) -> Vec<f32> {
        self.filters.0.push(self.source, 0);
        let value = *self.filters.0.transform().first().unwrap_or(&0.0);
        self.filters.1.push(value, 0);
        self.filters.1.transform()
    }

    fn postponable(&self) -> bool {
        false
    }
}
