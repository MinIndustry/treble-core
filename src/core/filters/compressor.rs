use crate::core::graph::{Entry, Filter};
#[cfg(feature = "meta")]
use rustic_derive::FilterMetaData;
use std::fmt;

/// A dynamics compressor that reduces the dynamic range of audio signals.
/// When the input exceeds the threshold, the output is compressed by the given ratio.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "meta", derive(FilterMetaData))]
pub struct Compressor {
    #[cfg_attr(feature = "meta", filter_source)]
    source: f32,
    /// Threshold in linear amplitude (0.0-1.0)
    #[cfg_attr(feature = "meta", filter_parameter(range, 0.0, 1.0, 0.5))]
    threshold: f32,
    /// Compression ratio (1.0 = no compression, higher = more compression)
    #[cfg_attr(feature = "meta", filter_parameter(range, 1.0, 20.0, 4.0))]
    ratio: f32,
    /// Attack time in seconds
    #[cfg_attr(feature = "meta", filter_parameter(range, 0.0001, 0.1, 0.01))]
    attack: f32,
    /// Release time in seconds
    #[cfg_attr(feature = "meta", filter_parameter(range, 0.01, 1.0, 0.1))]
    release: f32,
    /// Current envelope follower value
    envelope: f32,
    /// Sample rate for time-based calculations
    sample_rate: f32,
}

impl Default for Compressor {
    fn default() -> Self {
        Self {
            source: 0.0,
            threshold: 0.5,
            ratio: 4.0,
            attack: 0.01,
            release: 0.1,
            envelope: 0.0,
            sample_rate: 44100.0,
        }
    }
}

impl Entry for Compressor {
    fn push(&mut self, value: f32, _: usize) {
        self.source = value;
    }
}

impl fmt::Display for Compressor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Compressor Filter")
    }
}

impl Filter for Compressor {
    fn transform(&mut self) -> Vec<f32> {
        let input = self.source.abs();

        // --- Envelope follower ---
        let attack_coeff = (-1.0 / (self.attack * self.sample_rate)).exp();
        let release_coeff = (-1.0 / (self.release * self.sample_rate)).exp();

        if input > self.envelope {
            self.envelope = attack_coeff * (self.envelope - input) + input;
        } else {
            self.envelope = release_coeff * (self.envelope - input) + input;
        }

        // --- Gain computation ---
        let gain = if self.envelope > self.threshold {
            let excess = self.envelope / self.threshold;
            let compressed = excess.powf(1.0 - 1.0 / self.ratio);
            (compressed * self.threshold) / self.envelope
        } else {
            1.0
        };

        let output = self.source * gain;

        // One output port
        vec![output]
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
