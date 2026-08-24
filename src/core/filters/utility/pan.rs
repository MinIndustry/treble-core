use std::fmt;
use std::sync::Arc;

use crate::core::Block;
use crate::core::graph::{Entry, Filter};
use treble_derive::FilterMetaData;

/// Pans the output left or right
#[derive(FilterMetaData, Clone, Default)]
pub struct PanFilter {
    #[filter_source]
    source: Arc<Block>,
    #[filter_parameter(range, -1.0, 1.0, 0.01)]
    direction: f32,
}

impl PanFilter {
    pub fn new(direction: f32) -> Self {
        Self {
            source: Arc::new(Vec::new()),
            direction,
        }
    }
}

impl Entry for PanFilter {
    fn push(&mut self, block: Arc<Block>, _port: usize) {
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
        // Equal-power law: constant perceived loudness across the sweep.
        // The old linear law put the centre at -6 dB per channel, so panned
        // material audibly dipped through the middle.
        let theta = (self.direction.clamp(-1.0, 1.0) + 1.0) * std::f32::consts::FRAC_PI_4;
        let (right_gain, left_gain) = theta.sin_cos();

        // Serial on purpose — see GainFilter: rayon per tiny block is a
        // ~1000x pessimization, measured in benches/graph.rs.
        vec![
            self.source
                .iter()
                .map(|[l, r]| [*l * left_gain, *r * right_gain])
                .collect(),
        ]
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
