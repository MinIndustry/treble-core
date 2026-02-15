use crate::core::graph::{Entry, Filter};
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
    output: usize,
    sources: Vec<f32>,
    #[cfg_attr(feature = "meta", filter_parameter(list, inputs, float))]
    weights: Vec<f32>,
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
            output: outputs,
            sources: vec![0.0; inputs],
            weights: vec![1.0; inputs],
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

        vec![output; self.output]
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
