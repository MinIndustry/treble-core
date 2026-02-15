use std::fmt;

#[cfg(feature = "meta")]
use rustic_derive::FilterMetaData;

use crate::core::graph::{Entry, Filter};

/// A Tremolo filter, that changes sound amplitude on a sinusoid
/// basis.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "meta", derive(FilterMetaData))]
pub struct Tremolo {
    #[cfg_attr(feature = "meta", filter_source)]
    pub source: f32,
    pub phase: f32,
    #[cfg_attr(feature = "meta", filter_parameter(range, 0.0, 20.0, 1.0))]
    pub frequency: f32,
    #[cfg_attr(feature = "meta", filter_parameter(range, 0.0, 1.0, 0.5))]
    pub depth: f32,
    #[cfg_attr(feature = "meta", filter_parameter(range, 0.0, 192000.0, 44100.0))]
    pub sample_rate: f32,
}

impl Tremolo {
    pub fn new(frequency: f32, depth: f32, sample_rate: f32) -> Self {
        Self {
            source: 0.0,
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
    fn push(&mut self, value: f32, _port: usize) {
        self.source = value;
    }
}

impl Filter for Tremolo {
    fn transform(&mut self) -> Vec<f32> {
        self.phase += (2.0 * std::f32::consts::PI * self.frequency) / self.sample_rate;
        if self.phase > 2.0 * std::f32::consts::PI {
            self.phase -= 2.0 * std::f32::consts::PI;
        }
        let modulation = 1.0 - self.depth * (0.5 * (1.0 + self.phase.sin()));
        vec![self.source * modulation]
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
