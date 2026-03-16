use crate::core::graph::{Entry, Filter};
use crate::core::{Block, CHANNELS};
use rustic_derive::FilterMetaData;
use std::{f64::consts::PI, fmt};

/// Applies a bandpass filter to the input signal
/// source: <https://en.wikipedia.org/wiki/Digital_biquad_filter>
/// This structure implements the Direct form 2 from the above link.
#[derive(FilterMetaData, Clone, Debug, Default)]
pub struct ResonantBandpassFilter {
    #[filter_source]
    source: Block,
    b: [f64; 3], // b0, b1, b2
    a: [f64; 3], // a0, a1, a2
    /// Per-channel biquad delay elements: zs[ch][0..1]
    zs: [[f64; 2]; CHANNELS],
}

impl fmt::Display for ResonantBandpassFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Resonant bandpass filter")
    }
}

impl ResonantBandpassFilter {
    /// Resonant bandpass filter using a biquad design.
    /// Implemented from <http://musicweb.ucsd.edu/~trsmyth/filters/Bi_quadratic_Resonant_Filte.html>
    pub fn new(center_frequency: f32, quality: f32, sample_frequency: f32) -> Self {
        let period = 1.0 / sample_frequency;
        let bandwidth = center_frequency / quality;

        let r: f64 = (-PI * bandwidth as f64 * period as f64).exp();

        let gain = 1.0 - r;
        let b: [f64; 3] = [gain, 0.0, -gain * r];
        let a: [f64; 3] = [
            1.0,
            -2.0 * r * (2.0 * PI * center_frequency as f64 * period as f64).cos(),
            r * r,
        ];

        Self {
            source: Vec::new(),
            b,
            a,
            zs: [[0.0; 2]; CHANNELS],
        }
    }

    pub fn set_parameters(&mut self, center_frequency: f32, quality: f32, sample_frequency: f32) {
        let period = 1.0 / sample_frequency;
        let bandwidth = center_frequency / quality;

        let r: f64 = (-PI * bandwidth as f64 * period as f64).exp();

        let gain = 1.0 - r;
        self.b = [gain, 0.0, -gain * r];
        self.a = [
            1.0,
            -2.0 * r * (2.0 * PI * center_frequency as f64 * period as f64).cos(),
            r * r,
        ];
    }

    /// Resets the filter's internal state (delay elements).
    /// This is critical for percussive sounds where each hit should start with clean filter state.
    pub fn reset(&mut self) {
        self.zs = [[0.0; 2]; CHANNELS];
        self.source.clear();
    }
}

impl Entry for ResonantBandpassFilter {
    fn push(&mut self, block: Block, _port: usize) {
        self.source = block;
    }
}

impl Filter for ResonantBandpassFilter {
    fn transform(&mut self) -> Vec<Block> {
        let output: Block = self
            .source
            .iter()
            .map(|frame| {
                std::array::from_fn(|ch| {
                    let input = frame[ch] as f64;
                    let out = self.b[0] * input + self.zs[ch][0];
                    self.zs[ch][0] = self.b[2] * input - self.a[1] * out + self.zs[ch][1];
                    self.zs[ch][1] = -self.a[2] * out;
                    out as f32
                })
            })
            .collect();
        vec![output]
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
