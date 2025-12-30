use crate::core::graph::{AudioGraphElement, Entry, Filter};
use std::fmt;

#[cfg(feature = "meta")]
use rustic_derive::FilterMetaData;

/// A filter that take input from two sources and combines them into a single
/// output by adding them together.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "meta", derive(FilterMetaData))]
pub struct CombinatorFilter {
    #[cfg_attr(feature = "meta", filter_source)]
    inputs: usize,
    outputs: usize,
    sources: Vec<f32>,
    weights: Vec<f32>,
    index: usize,
}

impl Default for CombinatorFilter {
    fn default() -> Self {
        Self::new(1, 1)
    }
}

impl CombinatorFilter {
    pub fn new(inputs: usize, outputs: usize) -> Self {
        Self {
            inputs,
            outputs,
            sources: vec![0.0; inputs],
            weights: vec![1.0; inputs],
            index: 0,
        }
    }
}

impl Entry for CombinatorFilter {
    fn push(&mut self, value: f32, port: usize) {
        if port >= self.inputs {
            log::error!("Port {} out of bounds for CombinatorFilter", port);
        }
        self.sources[port] = value;
    }
}

impl fmt::Display for CombinatorFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Combinator Filter")
    }
}

impl Filter for CombinatorFilter {
    fn transform(&mut self) -> Vec<f32> {
        let output = self
            .sources
            .iter()
            .zip(&self.weights)
            .map(|(source, weight)| source * weight)
            .sum();

        vec![output; self.outputs]
    }

    fn postponable(&self) -> bool {
        false
    }
}

impl AudioGraphElement for CombinatorFilter {
    fn get_name(&self) -> &str {
        "Combinator"
    }

    fn get_index(&self) -> usize {
        self.index
    }

    fn set_index(&mut self, index: usize) {
        self.index = index;
    }
}
