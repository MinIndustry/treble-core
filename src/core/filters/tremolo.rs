use std::fmt;

#[cfg(feature = "meta")]
use rustic_derive::FilterMetaData;

use crate::core::graph::{AudioGraphElement, Entry, Filter};

/// A Tremolo filter, that changes sound amplitude on a sinusoid
/// basis.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "meta", derive(FilterMetaData))]
pub struct Tremolo {
    pub index: usize,
    #[cfg_attr(feature = "meta", filter_source)]
    pub source: f32,
    pub time: f32,
    #[cfg_attr(feature = "meta", filter_parameter(range, 0.0, 20.0, 1.0))]
    pub frequency: f32,
    #[cfg_attr(feature = "meta", filter_parameter(range, 0.0, 1.0, 0.5))]
    pub upper_range: f32,
    #[cfg_attr(feature = "meta", filter_parameter(range, 0.0, 1.0, 0.5))]
    pub lower_range: f32,
}

impl Tremolo {
    pub fn new(frequency: f32, min: f32, max: f32) -> Self {
        Self {
            index: 0,
            source: 0.0,
            time: 0.0,
            frequency,
            upper_range: max,
            lower_range: min,
        }
    }
}

impl fmt::Display for Tremolo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Tremolo: {}Hz ({}, {})",
            self.frequency, self.lower_range, self.upper_range
        )
    }
}

impl AudioGraphElement for Tremolo {
    fn get_name(&self) -> &str {
        "Tremolo"
    }

    fn get_index(&self) -> usize {
        self.index
    }

    fn set_index(&mut self, index: usize) {
        self.index = index;
    }
}

impl Entry for Tremolo {
    fn push(&mut self, value: f32, _port: usize) {
        self.source = value;
    }
}

impl Filter for Tremolo {
    fn transform(&mut self) -> Vec<f32> {
        self.time += 1.0 / 44100.0;
        vec![
            self.source
                * ((self.frequency * self.time).sin() * (self.upper_range - self.lower_range)
                    + (self.lower_range + self.upper_range) / 2.0),
        ]
    }

    fn postponable(&self) -> bool {
        false
    }
}
