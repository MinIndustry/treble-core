use crate::core::Block;
use crate::core::graph::{Entry, Filter};
use rustic_derive::FilterMetaData;
use std::fmt;

#[derive(FilterMetaData, Debug, Clone, Default)]
pub struct Clipper {
    #[filter_source]
    source: Block,
    #[filter_parameter(range, 0.0, 1.0, 0.5)]
    pub max_ampl: f32,
}

impl Clipper {
    pub fn new(max: f32) -> Self {
        Self {
            source: Vec::new(),
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
    fn push(&mut self, block: Block, _port: usize) {
        self.source = block;
    }
}

impl Filter for Clipper {
    fn transform(&mut self) -> Vec<Block> {
        let max = self.max_ampl;
        let output: Block = self
            .source
            .iter()
            .map(|frame| std::array::from_fn(|ch| frame[ch].clamp(-max, max)))
            .collect();
        vec![output]
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
