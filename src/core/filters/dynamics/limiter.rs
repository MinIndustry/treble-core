use std::fmt;
use std::sync::Arc;

use rustic_derive::FilterMetaData;

use crate::core::graph::{Entry, Filter};
use crate::core::{Block, CHANNELS};

/// A peak-tracking brick-wall limiter.
///
/// When the peak envelope exceeds `threshold`, gain is reduced to exactly
/// `threshold / envelope` (infinite ratio). Below threshold, gain is 1.0.
/// Uses the same per-channel envelope follower pattern as the [`Compressor`],
/// but with a hard ceiling instead of a configurable ratio.
#[derive(FilterMetaData, Clone, Debug)]
pub struct Limiter {
    #[filter_source]
    source: Arc<Block>,
    /// Output ceiling in linear amplitude (0.0–1.0)
    #[filter_parameter(range, 0.0, 1.0, 0.95)]
    threshold: f32,
    /// Attack time in seconds — how fast the limiter engages on a peak
    #[filter_parameter(range, 0.0001, 0.1, 0.001)]
    attack: f32,
    /// Release time in seconds — how fast the limiter recovers after a peak
    #[filter_parameter(range, 0.01, 1.0, 0.2)]
    release: f32,
    /// Per-channel peak envelope state
    envelope: [f32; CHANNELS],
    /// Sample rate used for coefficient calculation
    sample_rate: f32,
}

impl Default for Limiter {
    fn default() -> Self {
        Self {
            source: Arc::new(Vec::new()),
            threshold: 0.95,
            attack: 0.001,
            release: 0.2,
            envelope: [0.0; CHANNELS],
            sample_rate: 44100.0,
        }
    }
}

impl Entry for Limiter {
    fn push(&mut self, block: Arc<Block>, _port: usize) {
        self.source = block;
    }
}

impl fmt::Display for Limiter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Limiter Filter")
    }
}

impl Filter for Limiter {
    fn transform(&mut self) -> Vec<Block> {
        let attack_coeff = (-1.0 / (self.attack * self.sample_rate)).exp();
        let release_coeff = (-1.0 / (self.release * self.sample_rate)).exp();

        // iter() not par_iter(): envelope state is sequential across frames —
        // each frame's gain depends on the previous frame's envelope value.
        let output: Block = self
            .source
            .iter()
            .map(|frame| {
                std::array::from_fn(|ch| {
                    let input_abs = frame[ch].abs();
                    if input_abs > self.envelope[ch] {
                        self.envelope[ch] =
                            attack_coeff * (self.envelope[ch] - input_abs) + input_abs;
                    } else {
                        self.envelope[ch] =
                            release_coeff * (self.envelope[ch] - input_abs) + input_abs;
                    }
                    // Brick-wall gain: threshold / envelope (infinite ratio above threshold)
                    let gain = if self.envelope[ch] > self.threshold {
                        self.threshold / self.envelope[ch]
                    } else {
                        1.0
                    };
                    frame[ch] * gain
                })
            })
            .collect();

        vec![output]
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
