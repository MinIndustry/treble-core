use std::fmt;
use std::sync::Arc;

use treble_derive::FilterMetaData;

use crate::core::graph::{Entry, Filter};
use crate::core::{Block, CHANNELS};
/// A dynamics compressor that reduces the dynamic range of audio signals.
/// When the input exceeds the threshold, the output is compressed by the given ratio.
#[derive(FilterMetaData, Clone, Debug)]
pub struct Compressor {
    #[filter_source]
    source: Arc<Block>,
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
    #[filter_parameter(range, 1.0, 192000.0, 44100.0)]
    sample_rate: f32,
}

impl Default for Compressor {
    fn default() -> Self {
        Self {
            source: Arc::new(Vec::new()),
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
    fn push(&mut self, block: Arc<Block>, _port: usize) {
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
                    let gain = if self.envelope[ch] > self.threshold && self.ratio > 1.0 {
                        (self.threshold / self.envelope[ch]).powf(1.0 - 1.0 / self.ratio)
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

#[cfg(test)]
mod tests {
    use super::*;
    use treble_meta::MetaFilter;
    #[test]
    fn unity_ratio_is_unity_and_zero_threshold_stays_finite() {
        let mut filter = Compressor::default();
        filter.set_parameter("ratio", 1.0);
        let input = vec![[2.0, -2.0]; 1024];
        filter.push(Arc::new(input.clone()), 0);
        assert_eq!(filter.transform()[0], input);
        filter.set_parameter("ratio", 4.0);
        filter.set_parameter("threshold", 0.0);
        filter.push(Arc::new(vec![[1.0; 2]; 10]), 0);
        assert_eq!(filter.transform()[0], vec![[0.0; 2]; 10]);
    }
    #[test]
    fn ratio_matches_the_settled_output_curve_at_the_host_sample_rate() {
        for rate in [22050.0, 48000.0, 96000.0] {
            let mut filter = Compressor::default();
            assert!(filter.set_parameter("sample_rate", rate));
            filter.set_parameter("attack", 0.0001);
            filter.set_parameter("threshold", 0.5);
            filter.set_parameter("ratio", 4.0);
            filter.push(Arc::new(vec![[2.0; 2]; 2048]), 0);
            let output = filter.transform();
            let expected = 0.5 * 4.0_f32.powf(0.25);
            assert!((output[0].last().unwrap()[0] - expected).abs() < 0.0001);
        }
    }
}
