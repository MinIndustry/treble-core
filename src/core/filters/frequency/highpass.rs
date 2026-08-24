use std::fmt;
use std::sync::Arc;

use treble_derive::FilterMetaData;

use super::svf::{DEFAULT_RESONANCE, Svf};
use crate::core::Block;
use crate::core::graph::{Entry, Filter};

#[derive(FilterMetaData, Clone, Debug)]
/// Resonant high-pass filter: a 2-pole state-variable filter, 12 dB/octave,
/// with a `resonance` (Q) control that peaks the response at the cutoff.
/// This replaces a one-pole design, so it strips more low end below the
/// cutoff than earlier versions of the same filter.
pub struct HighPassFilter {
    #[filter_source]
    source: Arc<Block>,
    #[filter_parameter(range, 1.0, 20000.0, 1000.0)]
    cutoff_frequency: f32,
    // Range and default must match svf::{MIN,MAX,DEFAULT}_RESONANCE; the
    // derive parses literals out of this attribute and cannot see constants.
    #[filter_parameter(range, 0.5, 20.0, 0.70710678)]
    resonance: f32,
    // Registered so the engine injects its real rate at build time; a plain
    // field would keep the default and mistune every cutoff.
    #[filter_parameter(range, 1.0, 192000.0, 44100.0)]
    sample_rate: f32,
    svf: Svf,
    /// The `(cutoff, resonance, rate)` the coefficients were built for. The
    /// derived `set_parameter` assigns fields without recomputing, so
    /// `transform` is the only place that can notice a change.
    coefficients_for: (f32, f32, f32),
}

impl Default for HighPassFilter {
    fn default() -> Self {
        Self::new(1000.0, 44100.0)
    }
}

impl HighPassFilter {
    /// A neutral high-pass: Butterworth Q, no resonant peak.
    pub fn new(cutoff_frequency: f32, sample_rate: f32) -> Self {
        Self::with_resonance(cutoff_frequency, DEFAULT_RESONANCE, sample_rate)
    }

    pub fn with_resonance(cutoff_frequency: f32, resonance: f32, sample_rate: f32) -> Self {
        Self {
            source: Arc::new(Vec::new()),
            cutoff_frequency,
            resonance,
            sample_rate,
            svf: Svf::new(cutoff_frequency, resonance, sample_rate),
            coefficients_for: (cutoff_frequency, resonance, sample_rate),
        }
    }
}

impl Entry for HighPassFilter {
    fn push(&mut self, block: Arc<Block>, _port: usize) {
        self.source = block;
    }
}

impl fmt::Display for HighPassFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "High Pass Filter - {}Hz, Q: {}",
            self.cutoff_frequency, self.resonance
        )
    }
}

impl Filter for HighPassFilter {
    fn transform(&mut self) -> Vec<Block> {
        let parameters = (self.cutoff_frequency, self.resonance, self.sample_rate);
        if parameters != self.coefficients_for {
            self.svf
                .set_coefficients(parameters.0, parameters.1, parameters.2);
            self.coefficients_for = parameters;
        }

        let output: Block = self
            .source
            .iter()
            .map(|frame| std::array::from_fn(|channel| self.svf.high_pass(channel, frame[channel])))
            .collect();

        self.svf.reset_if_unstable();
        vec![output]
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::filters::frequency::svf::MAX_RESONANCE;
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
        let sum: f64 = tail
            .iter()
            .map(|frame| (frame[0] as f64) * (frame[0] as f64))
            .sum();
        (sum / tail.len() as f64).sqrt() as f32
    }

    #[test]
    fn a_low_tone_is_attenuated_far_more_than_a_high_one() {
        let rate = 48_000.0;
        let frames = 24_000;
        let mut low = HighPassFilter::new(2_000.0, rate);
        low.push(sine(125.0, rate, frames), 0);
        let low_rms = settled_rms(&low.transform().remove(0));

        let mut high = HighPassFilter::new(2_000.0, rate);
        high.push(sine(10_000.0, rate, frames), 0);
        let high_rms = settled_rms(&high.transform().remove(0));

        assert!(
            high_rms > low_rms * 100.0,
            "10 kHz at {high_rms} vs 125 Hz at {low_rms} — the slope is too shallow"
        );
    }

    #[test]
    fn resonance_lifts_the_cutoff_above_the_neutral_response() {
        let rate = 48_000.0;
        let frames = 24_000;
        let cutoff = 1_000.0;

        let mut neutral = HighPassFilter::new(cutoff, rate);
        neutral.push(sine(cutoff, rate, frames), 0);
        let neutral_rms = settled_rms(&neutral.transform().remove(0));

        let mut resonant = HighPassFilter::with_resonance(cutoff, 10.0, rate);
        resonant.push(sine(cutoff, rate, frames), 0);
        let resonant_rms = settled_rms(&resonant.transform().remove(0));

        let gain = 20.0 * (resonant_rms / neutral_rms).log10();
        assert!(
            gain > 15.0,
            "Q=10 at the cutoff should be well over 15 dB up, measured {gain:.1} dB"
        );
    }

    #[test]
    fn a_fast_full_range_sweep_at_maximum_resonance_stays_bounded() {
        let rate = 48_000.0;
        let mut filter = HighPassFilter::with_resonance(1_000.0, MAX_RESONANCE, rate);
        for block_index in 0..800 {
            let position = (block_index as f32 / 200.0).fract();
            let sweep = if position < 0.5 {
                position * 2.0
            } else {
                2.0 - position * 2.0
            };
            let cutoff = 20.0 * (20_000.0f32 / 20.0).powf(sweep);
            assert!(filter.set_parameter("cutoff_frequency", cutoff));

            let block: Block = (0..256)
                .map(|index| {
                    let frame = block_index * 256 + index;
                    let phase = std::f32::consts::TAU * 440.0 * frame as f32 / rate;
                    [phase.sin(); crate::core::CHANNELS]
                })
                .collect();
            filter.push(Arc::new(block), 0);
            for frame in filter.transform().remove(0) {
                for value in frame {
                    assert!(
                        value.is_finite() && value.abs() < 64.0,
                        "block {block_index} at cutoff {cutoff}: {value}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_engine_can_inject_every_parameter_by_name() {
        let filter = HighPassFilter::default();
        for name in ["cutoff_frequency", "resonance", "sample_rate"] {
            assert!(
                filter.supports_parameter(name),
                "'{name}' must be settable from an FxSpec"
            );
        }
    }
}
