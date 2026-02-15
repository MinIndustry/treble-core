use std::fmt;

#[cfg(feature = "meta")]
use rustic_derive::FilterMetaData;

use crate::core::graph::{Entry, Filter};

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "meta", derive(FilterMetaData))]
pub struct Clipper {
    #[cfg_attr(feature = "meta", filter_source)]
    pub source: f32,
    #[cfg_attr(feature = "meta", filter_parameter(range, 0.0, 1.0, 0.5))]
    pub max_ampl: f32,
}

impl Clipper {
    pub fn new(max: f32) -> Self {
        Self {
            source: 0.0,
            max_ampl: max,
        }
    }
}

impl fmt::Display for Clipper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Clipper: max ampl: {}", self.max_ampl)
    }
}

impl Entry for Clipper {
    fn push(&mut self, value: f32, _port: usize) {
        self.source = value;
    }
}

impl Filter for Clipper {
    fn transform(&mut self) -> Vec<f32> {
        vec![self.source.clamp(-self.max_ampl, self.max_ampl)]
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
