use crate::core::graph::{Entry, Filter};
use crate::core::{Block, CHANNELS, Frame};
use rustic_meta::{FilterInfo, FilterInput, MetaFilter};
use std::fmt;

/// A filter that takes input from N sources and combines them into M outputs
/// by adding weighted sources together.
#[derive(Clone, Debug)]
pub struct CombinatorFilter {
    inputs: usize,
    output: usize,
    sources: Vec<Block>,
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
            sources: vec![Vec::new(); inputs],
            weights: vec![1.0; inputs],
        }
    }
}

impl Entry for CombinatorFilter {
    fn push(&mut self, block: Block, port: usize) {
        if port >= self.inputs {
            log::error!("Port {} out of bounds for CombinatorFilter", port);
            return;
        }
        self.sources[port] = block;
    }
}

impl fmt::Display for CombinatorFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Combinator Filter")
    }
}

impl MetaFilter for CombinatorFilter {
    fn set_parameter(&mut self, name: &str, _value: f32) {
        log::warn!("Unknown parameter '{}' for CombinatorFilter", name);
    }

    fn metadata() -> FilterInfo {
        FilterInfo {
            name: "CombinatorFilter",
            description: "Combines multiple audio inputs into outputs with optional weighting",
            inputs: vec![FilterInput {
                label: None,
                parameter: None,
            }],
            outputs: 1,
        }
    }
}

impl crate::meta::traits::FilterFactory for CombinatorFilter {
    fn create_instance(&self) -> Box<dyn crate::core::graph::Filter> {
        Box::from(CombinatorFilter::default()) as Box<dyn crate::core::graph::Filter>
    }
}

impl Filter for CombinatorFilter {
    fn transform(&mut self) -> Vec<Block> {
        // Find the block size from the first non-empty source
        let block_size = self
            .sources
            .iter()
            .find(|s| !s.is_empty())
            .map(|s| s.len())
            .unwrap_or(0);

        if block_size == 0 {
            return vec![Vec::new(); self.output];
        }

        // Sum all weighted sources per frame per channel
        let output: Block = (0..block_size)
            .map(|i| {
                let mut frame: Frame = [0.0; CHANNELS];
                for (src, &weight) in self.sources.iter().zip(self.weights.iter()) {
                    if let Some(src_frame) = src.get(i) {
                        for ch in 0..CHANNELS {
                            frame[ch] += src_frame[ch] * weight;
                        }
                    }
                }
                frame
            })
            .collect();

        vec![output; self.output]
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
