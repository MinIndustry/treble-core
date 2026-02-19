use crate::core::graph::{Entry, Filter};
use crate::core::Block;
use rustic_derive::FilterMetaData;
use std::fmt;

/// A filter that returns the input value multiplied by a constant factor.
/// Note: a factor < 1.0 will attenuate the input signal, while a factor > 1.0
/// will amplify it.
#[derive(FilterMetaData, Clone, Debug, Default)]
pub struct GainFilter {
    #[filter_source]
    source: Block,
    #[filter_parameter(float, 1.0)]
    factor: f32,
}

impl GainFilter {
    pub fn new(factor: f32) -> Self {
        Self {
            source: Vec::new(),
            factor,
        }
    }
}

impl Entry for GainFilter {
    fn push(&mut self, block: Block, _port: usize) {
        self.source = block;
    }
}

impl fmt::Display for GainFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Gain Filter - factor: {}", self.factor)
    }
}

impl Filter for GainFilter {
    fn transform(&mut self) -> Vec<Block> {
        let factor = self.factor;
        let output: Block = self.source
            .iter()
            .map(|frame| std::array::from_fn(|ch| frame[ch] * factor))
            .collect();
        vec![output]
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
