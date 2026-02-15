use std::fmt;

#[cfg(feature = "meta")]
use rustic_derive::FilterMetaData;

use crate::core::graph::{Entry, Filter};

use super::{HighPassFilter, LowPassFilter};

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "meta", derive(FilterMetaData))]
/// Bandpass filter using a high-pass and low-pass filter
pub struct BandPass {
    #[cfg_attr(feature = "meta", filter_parameter(range, 0.0, 20000.0, 1000.0))]
    pub low: f32,
    #[cfg_attr(feature = "meta", filter_parameter(range, 0.0, 20000.0, 1000.0))]
    pub high: f32,
    #[cfg_attr(feature = "meta", filter_parameter(range, 0.0, 192000.0, 44100.0))]
    pub sample_rate: f32,
    pub filters: (HighPassFilter, LowPassFilter),
    #[cfg_attr(feature = "meta", filter_source)]
    pub source: f32,
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
            source: 0.0,
        }
    }
}

impl fmt::Display for BandPass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bandpass filter ({} - {})", self.low, self.high)
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

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
