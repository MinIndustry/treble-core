use std::fmt;

use crate::core::graph::{AudioGraphElement, Entry, Filter};

/// A Tremolo filter, that changes sound amplitude on a sinusoid
/// basis.
#[derive(Debug, Clone)]
pub struct Tremolo {
    pub index: usize,
    pub source: f32,
    pub time: f32,
    pub frequency: f32,
    pub ampl_limits: (f32, f32),
}

impl Tremolo {
    pub fn new(frequency: f32, min: f32, max: f32) -> Self {
        Self {
            index: 0,
            source: 0.0,
            time: 0.0,
            frequency,
            ampl_limits: (min, max),
        }
    }
}

impl fmt::Display for Tremolo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Tremolo: {}Hz ({}, {})",
            self.frequency, self.ampl_limits.0, self.ampl_limits.1
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
                * ((self.frequency * self.time).sin() * (self.ampl_limits.1 - self.ampl_limits.0)
                    + (self.ampl_limits.0 + self.ampl_limits.1) / 2.0),
        ]
    }

    fn postponable(&self) -> bool {
        false
    }
}
