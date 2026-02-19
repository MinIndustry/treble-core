use crate::core::graph::{Entry, Filter};
use crate::core::{Block, CHANNELS};
use rustic_derive::FilterMetaData;
use std::fmt;

/// A dynamics compressor that reduces the dynamic range of audio signals.
/// When the input exceeds the threshold, the output is compressed by the given ratio.
#[derive(FilterMetaData, Clone, Debug)]
pub struct Compressor {
    #[filter_source]
    source: Block,
    /// Threshold in linear amplitude (0.0-1.0)
    #[filter_parameter(range, 0.0, 1.0, 0.5)]
    threshold: f32,
    /// Compression ratio (1.0 = no compression, higher = more compression)
    #[filter_parameter(range, 1.0, 20.0, 4.0)]
    ratio: f32,
    /// Attack time in seconds
    #[filter_parameter(range, 0.0001, 0.1, 0.01)]
    attack: f32,
    /// Release time in seconds
    #[filter_parameter(range, 0.01, 1.0, 0.1)]
    release: f32,
    /// Per-channel envelope follower values
    envelope: [f32; CHANNELS],
    /// Sample rate for time-based calculations
    sample_rate: f32,
}

impl Default for Compressor {
    fn default() -> Self {
        Self {
            source: Vec::new(),
            threshold: 0.5,
            ratio: 4.0,
            attack: 0.01,
            release: 0.1,
            envelope: [0.0; CHANNELS],
            sample_rate: 44100.0,
        }
    }
}

impl Entry for Compressor {
    fn push(&mut self, block: Block, _port: usize) {
        self.source = block;
    }
}

impl fmt::Display for Compressor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Compressor Filter")
    }
}

impl Filter for Compressor {
    fn transform(&mut self) -> Vec<Block> {
        let attack_coeff = (-1.0 / (self.attack * self.sample_rate)).exp();
        let release_coeff = (-1.0 / (self.release * self.sample_rate)).exp();

        let output: Block = self.source
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
                    let gain = if self.envelope[ch] > self.threshold {
                        let excess = self.envelope[ch] / self.threshold;
                        let compressed = excess.powf(1.0 - 1.0 / self.ratio);
                        (compressed * self.threshold) / self.envelope[ch]
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
