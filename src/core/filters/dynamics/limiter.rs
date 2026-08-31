use std::fmt;
use std::sync::Arc;

use treble_derive::FilterMetaData;

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
    /// Per-channel smoothed gain, carried between blocks. Unity until
    /// something asks for reduction.
    gain: [f32; CHANNELS],
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
            gain: [1.0; CHANNELS],
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

                    // What this sample needs to sit under the ceiling. Deriving
                    // the gain from the envelope alone made `attack` a lag
                    // rather than an attack: a transient passed at up to twice
                    // the threshold for roughly ten times the attack time
                    // before the envelope caught up.
                    let required = if input_abs > self.threshold {
                        self.threshold / input_abs
                    } else {
                        1.0
                    };

                    self.envelope[ch] =
                        input_abs.max(release_coeff * (self.envelope[ch] - input_abs) + input_abs);

                    let coeff = if required < self.gain[ch] {
                        attack_coeff
                    } else {
                        release_coeff
                    };
                    self.gain[ch] = coeff * (self.gain[ch] - required) + required;

                    // Smoothing may lag; `required` is the hard guarantee, and
                    // taking the smaller is what makes this a ceiling.
                    frame[ch] * self.gain[ch].min(required)
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
mod ceiling_tests {
    use super::*;

    fn feed(limiter: &mut Limiter, samples: &[f32]) -> Vec<f32> {
        let block: Block = samples.iter().map(|&v| [v, v]).collect();
        limiter.push(Arc::new(block), 0);
        limiter.transform()[0].iter().map(|f| f[0]).collect()
    }

    /// The ceiling holds from the first sample, as `audio_sink` does.
    ///
    /// The gain used to come from a lagging envelope, so `attack` behaved as a
    /// delay before limiting rather than as the speed of it, and a transient
    /// escaped at up to twice the threshold.
    #[test]
    fn nothing_escapes_the_threshold() {
        let mut limiter = Limiter::default();
        let threshold = limiter.threshold;
        let out = feed(&mut limiter, &[3.0; 512]);
        let worst = out.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(worst <= threshold + 1e-6, "a step to 3.0 peaked at {worst}");
        assert!(
            (out[0].abs() - threshold).abs() < 1e-6,
            "the first sample must already be limited, got {}",
            out[0]
        );
    }

    /// Material that already fits passes through unchanged.
    #[test]
    fn quiet_material_is_untouched() {
        let mut limiter = Limiter::default();
        let input: Vec<f32> = (0..256).map(|i| 0.3 * (i as f32 * 0.1).sin()).collect();
        let out = feed(&mut limiter, &input);
        for (got, want) in out.iter().zip(&input) {
            assert!((got - want).abs() < 1e-6, "expected {want}, got {got}");
        }
    }
}
