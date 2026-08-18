use std::sync::Arc;
use std::{f64::consts::PI, fmt};

use treble_derive::FilterMetaData;

use crate::core::graph::{Entry, Filter};
use crate::core::{Block, CHANNELS};

/// Applies a bandpass filter to the input signal
/// source: <https://en.wikipedia.org/wiki/Digital_biquad_filter>
/// This structure implements the Direct form 2 from the above link.
#[derive(FilterMetaData, Clone, Debug)]
pub struct ResonantBandpassFilter {
    #[filter_source]
    source: Arc<Block>,
    #[filter_parameter(range, 20.0, 20000.0, 1000.0)]
    center_frequency: f32,
    #[filter_parameter(range, 0.1, 100.0, 1.0)]
    quality: f32,
    #[filter_parameter(range, 1.0, 192000.0, 44100.0)]
    sample_rate: f32,
    b: [f64; 3], // b0, b1, b2
    a: [f64; 3], // a0, a1, a2
    /// The (center_frequency, quality, sample_rate) the current coefficients
    /// were computed for; transform() recomputes when the params have changed.
    coeffs_for: (f32, f32, f32),
    /// Per-channel biquad delay elements: zs[ch][0..1]
    zs: [[f64; 2]; CHANNELS],
}

impl Default for ResonantBandpassFilter {
    fn default() -> Self {
        Self::new(1000.0, 1.0, 44100.0)
    }
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
        let (b, a) = Self::compute_coefficients(center_frequency, quality, sample_frequency);

        Self {
            source: Arc::new(Vec::new()),
            center_frequency,
            quality,
            sample_rate: sample_frequency,
            b,
            a,
            coeffs_for: (center_frequency, quality, sample_frequency),
            zs: [[0.0; 2]; CHANNELS],
        }
    }

    fn compute_coefficients(
        center_frequency: f32,
        quality: f32,
        sample_frequency: f32,
    ) -> ([f64; 3], [f64; 3]) {
        let period = 1.0 / sample_frequency;
        let bandwidth = center_frequency / quality;

        let r: f64 = (-PI * bandwidth as f64 * period as f64).exp();

        let gain = 1.0 - r;
        let b = [gain, 0.0, -gain * r];
        let a = [
            1.0,
            -2.0 * r * (2.0 * PI * center_frequency as f64 * period as f64).cos(),
            r * r,
        ];
        (b, a)
    }

    pub fn set_parameters(&mut self, center_frequency: f32, quality: f32, sample_frequency: f32) {
        self.center_frequency = center_frequency;
        self.quality = quality;
        self.sample_rate = sample_frequency;
        let (b, a) = Self::compute_coefficients(center_frequency, quality, sample_frequency);
        self.b = b;
        self.a = a;
        self.coeffs_for = (center_frequency, quality, sample_frequency);
    }

    /// Resets the filter's internal state (delay elements).
    /// This is critical for percussive sounds where each hit should start with clean filter state.
    pub fn reset(&mut self) {
        self.zs = [[0.0; 2]; CHANNELS];
        self.source = Arc::new(Vec::new());
    }
}

impl Entry for ResonantBandpassFilter {
    fn push(&mut self, block: Arc<Block>, _port: usize) {
        self.source = block;
    }
}

impl Filter for ResonantBandpassFilter {
    fn transform(&mut self) -> Vec<Block> {
        // Params may have been changed through the derived set_parameter,
        // which assigns fields without recomputing the biquad coefficients.
        let params = (self.center_frequency, self.quality, self.sample_rate);
        if params != self.coeffs_for {
            let (b, a) = Self::compute_coefficients(params.0, params.1, params.2);
            self.b = b;
            self.a = a;
            self.coeffs_for = params;
        }

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
