use rayon::prelude::*;
use std::fmt;

use crate::core::Block;
use crate::core::graph::{Entry, Filter};
use rustic_derive::FilterMetaData;

/// Pans the output left or right
#[derive(FilterMetaData, Clone, Default)]
pub struct PanFilter {
    #[filter_source]
    source: Block,
    #[filter_parameter(range, -1.0, 1.0, 0.01)]
    direction: f32,
}

impl PanFilter {
    pub fn new(direction: f32) -> Self {
        Self {
            source: Vec::new(),
            direction: direction,
        }
    }
}

impl Entry for PanFilter {
    fn push(&mut self, block: Block, _port: usize) {
        self.source = block;
    }
}

impl fmt::Display for PanFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Pan Filter - {}", self.direction)
    }
}

impl fmt::Debug for PanFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PanFilter {{ direction: {} }}", self.direction)
    }
}

impl Filter for PanFilter {
    fn transform(&mut self) -> Vec<Block> {
        let left_gain = (1.0 - self.direction) * 0.5;
        let right_gain = (1.0 + self.direction) * 0.5;

        // Use rayon for // execution
        vec![
            self.source
                .par_iter()
                .map(|[l, r]| [*l * left_gain, *r * right_gain])
                .collect(),
        ]
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
