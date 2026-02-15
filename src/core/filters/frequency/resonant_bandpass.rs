use std::{f64::consts::PI, fmt};

#[cfg(feature = "meta")]
use rustic_derive::FilterMetaData;

use crate::core::graph::{Entry, Filter};

/// Applies a bandpass filter to the input signal
/// source: <https://en.wikipedia.org/wiki/Digital_biquad_filter>
/// This structure implements the Direct form 2 from the above link.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "meta", derive(FilterMetaData))]
pub struct ResonantBandpassFilter {
    #[cfg_attr(feature = "meta", filter_source)]
    source: f32,
    #[cfg_attr(feature = "meta", filter_parameter(vec, 3, float))]
    b: [f64; 3], // b0, b1, b2
    #[cfg_attr(feature = "meta", filter_parameter(vec, 3, float))]
    a: [f64; 3], // a0, a1, a2
    #[cfg_attr(feature = "meta", filter_parameter(vec, 2, float))]
    zs: [f64; 2], // Delay elements z1, z2 for the filter
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
            source: 0.0,
            b,
            a,
            zs: [0.0; 2],
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
    /// Without reset, residual energy from previous notes can cause tonal artifacts and
    /// reduced transient clarity.
    pub fn reset(&mut self) {
        self.zs = [0.0; 2];
        self.source = 0.0;
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

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
