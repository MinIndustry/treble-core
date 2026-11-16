use std::fmt;
use std::sync::Arc;

use treble_derive::FilterMetaData;

use super::{HighPassFilter, LowPassFilter};
use crate::core::Block;
use crate::core::graph::{Entry, Filter};

#[derive(FilterMetaData, Debug, Clone)]
/// Bandpass filter using a high-pass and low-pass filter
pub struct BandPass {
    #[filter_parameter(range, 1.0, 20000.0, 1000.0, scale = logarithmic)]
    pub low: f32,
    #[filter_parameter(range, 1.0, 20000.0, 1000.0, scale = logarithmic)]
    pub high: f32,
    // Registered so the engine injects its real rate at build time; a plain
    // field would keep the default and mistune both cutoffs.
    #[filter_parameter(range, 1.0, 192000.0, 44100.0, access = engine)]
    pub sample_rate: f32,
    /// The high-pass at `low` and the low-pass at `high`. They are retuned
    /// from the fields above on every `transform`: the derived `set_parameter`
    /// assigns `low`, `high` and `sample_rate` without touching them, and
    /// building them once in `new` left every spec-built band pass fixed at
    /// the default 1 kHz whatever its parameters said.
    pub filters: (HighPassFilter, LowPassFilter),
    #[filter_source]
    pub source: Arc<Block>,
}

impl Default for BandPass {
    fn default() -> Self {
        Self::new(1000.0, 1000.0, 44100.0)
    }
}

impl BandPass {
    pub fn new(low: f32, high: f32, sample_rate: f32) -> Self {
        Self {
            low,
            high,
            sample_rate,
            filters: (
                HighPassFilter::new(low, sample_rate),
                LowPassFilter::new(high, sample_rate),
            ),
            source: Arc::new(Vec::new()),
        }
    }
}

impl fmt::Display for BandPass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bandpass filter ({} - {})", self.low, self.high)
    }
}

impl Entry for BandPass {
    fn push(&mut self, block: Arc<Block>, _port: usize) {
        self.source = block;
    }
}

impl Filter for BandPass {
    fn transform(&mut self) -> Vec<Block> {
        // A band given upside down is still the band between the two edges.
        let (low, high) = (self.low.min(self.high), self.low.max(self.high));
        self.filters.0.retune(low, self.sample_rate);
        self.filters.1.retune(high, self.sample_rate);
        self.filters.0.push(Arc::clone(&self.source), 0);
        let hp_output = self.filters.0.transform();
        self.filters.1.push(
            Arc::new(hp_output.into_iter().next().unwrap_or_default()),
            0,
        );
        self.filters.1.transform()
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use treble_meta::MetaFilter;

    fn sine(frequency: f32, sample_rate: f32, frames: usize) -> Arc<Block> {
        Arc::new(
            (0..frames)
                .map(|index| {
                    let phase = std::f32::consts::TAU * frequency * index as f32 / sample_rate;
                    [phase.sin(); crate::core::CHANNELS]
                })
                .collect(),
        )
    }

    fn settled_rms(block: &Block) -> f32 {
        let tail = &block[block.len() / 2..];
        let sum: f64 = tail.iter().map(|f| (f[0] as f64) * (f[0] as f64)).sum();
        (sum / tail.len() as f64).sqrt() as f32
    }

    /// A band pass built the way a spec builds it: defaults, then parameters.
    fn from_parameters(low: f32, high: f32, rate: f32) -> BandPass {
        let mut filter = BandPass::default();
        assert!(filter.set_parameter("low", low));
        assert!(filter.set_parameter("high", high));
        assert!(filter.set_parameter("sample_rate", rate));
        filter
    }

    fn response(filter: &mut BandPass, frequency: f32, rate: f32) -> f32 {
        filter.push(sine(frequency, rate, 24_000), 0);
        settled_rms(&filter.transform().remove(0))
    }

    #[test]
    fn parameters_set_after_construction_move_the_band() {
        let rate = 48_000.0;
        // A 5-6 kHz band passes 5.5 kHz and stops 1 kHz; before the fix it was
        // fixed at the 1 kHz default and did the opposite.
        let pass = response(&mut from_parameters(5_000.0, 6_000.0, rate), 5_500.0, rate);
        let stop = response(&mut from_parameters(5_000.0, 6_000.0, rate), 1_000.0, rate);
        assert!(pass > stop * 4.0, "5.5 kHz at {pass} vs 1 kHz at {stop}");
    }

    #[test]
    fn a_reversed_band_is_the_same_band() {
        let rate = 48_000.0;
        let a = response(&mut from_parameters(400.0, 4_000.0, rate), 1_200.0, rate);
        let b = response(&mut from_parameters(4_000.0, 400.0, rate), 1_200.0, rate);
        assert!((a - b).abs() < 1e-6);
    }

    #[test]
    fn the_rate_is_an_engine_parameter() {
        let rate = 22_050.0;
        let mut filter = from_parameters(5_000.0, 6_000.0, rate);
        assert_eq!(filter.sample_rate, rate);
        let pass = response(&mut filter, 5_500.0, rate);
        assert!(
            pass > 0.3,
            "5.5 kHz should pass a 5-6 kHz band at {rate} Hz, got {pass}"
        );
    }
}
