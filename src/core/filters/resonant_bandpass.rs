use std::{f64::consts::PI, fmt};

use crate::core::graph::{AudioGraphElement, Entry, Filter};

#[cfg(feature = "meta")]
use super::{FilterMetadata, Metadata};

/// Applies a bandpass filter to the input signal
/// source: https://en.wikipedia.org/wiki/Digital_biquad_filter
/// This structure implements the Direct form 2 from the above link.
#[derive(Clone, Debug)]
pub struct ResonantBandpassFilter {
    source: f32,
    index: usize,
    b: [f64; 3],  // b0, b1, b2
    a: [f64; 3],  // a0, a1, a2
    zs: [f64; 2], // Delay elements z1, z2 for the filter
}

impl fmt::Display for ResonantBandpassFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Resonant bandpass filter")
    }
}

impl ResonantBandpassFilter {
    /// Resonant bandpass filter using a biquad design.
    /// Implemented from http://musicweb.ucsd.edu/~trsmyth/filters/Bi_quadratic_Resonant_Filte.html
    pub fn new(center_frequency: f32, quality: f32, sample_frequency: f32) -> Self {
        // magnitude of the roots

        let period = 1.0 / sample_frequency;
        let bandwidth = center_frequency / quality;

        let r: f64 = (-PI * bandwidth as f64 * period as f64).exp();

        let b: [f64; 3] = [1.0, 0.0, -r];
        let a: [f64; 3] = [
            1.0,
            -2.0 * r * (2.0 * PI * center_frequency as f64 * period as f64).cos(),
            r * r,
        ];

        Self {
            source: 0.0,
            index: 0,
            b,
            a,
            zs: [0.0; 2],
        }
    }
}

impl Entry for ResonantBandpassFilter {
    fn push(&mut self, value: f32, _port: usize) {
        self.source = value;
    }
}

impl Filter for ResonantBandpassFilter {
    fn transform(&mut self) -> Vec<f32> {
        let output = self.b[0] * self.source as f64 + self.zs[0];
        self.zs[0] = self.b[2] * self.source as f64 - self.a[1] * output + self.zs[1];
        self.zs[1] = -self.a[2] * output;

        vec![output as f32]
    }

    fn postponable(&self) -> bool {
        false
    }
}

impl AudioGraphElement for ResonantBandpassFilter {
    fn get_name(&self) -> &str {
        "Resonant Bandpass Filter"
    }

    fn get_index(&self) -> usize {
        self.index
    }

    fn set_index(&mut self, index: usize) {
        self.index = index;
    }
}
