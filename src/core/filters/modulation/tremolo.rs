use std::fmt;
use std::sync::Arc;

use treble_derive::FilterMetaData;

use crate::core::Block;
use crate::core::graph::{Entry, Filter};

/// A Tremolo filter, that changes sound amplitude on a sinusoid
/// basis.
#[derive(FilterMetaData, Debug, Clone)]
pub struct Tremolo {
    #[filter_source]
    source: Arc<Block>,
    /// Normalised phase in `0.0..1.0`, at f64 so a slow LFO does not drift.
    phase: f64,
    #[filter_parameter(range, 0.0, 20.0, 1.0)]
    pub frequency: f32,
    #[filter_parameter(range, 0.0, 1.0, 0.5)]
    pub depth: f32,
    // Registered so the engine injects its real rate at build time. As a plain
    // field it stayed at the derived default of 0, which collapsed the phase
    // increment's divisor to 1 and detuned the LFO by the sample rate.
    #[filter_parameter(range, 1.0, 192000.0, 44100.0)]
    pub sample_rate: f32,
}

impl Default for Tremolo {
    fn default() -> Self {
        Self::new(1.0, 0.5, 44100.0)
    }
}

impl Tremolo {
    pub fn new(frequency: f32, depth: f32, sample_rate: f32) -> Self {
        Self {
            source: Arc::new(Vec::new()),
            phase: 0.0,
            frequency,
            depth,
            sample_rate,
        }
    }
}

impl fmt::Display for Tremolo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tremolo: {}Hz, depth: {}", self.frequency, self.depth)
    }
}

impl Entry for Tremolo {
    fn push(&mut self, block: Arc<Block>, _port: usize) {
        self.source = block;
    }
}

impl Filter for Tremolo {
    /// Timeline-anchored, like `AutoPanFilter`: a hot-swapped replacement
    /// resumes the wobble at the phase its predecessor had.
    fn on_transport(&mut self, frame: u64) {
        self.phase =
            (frame as f64 * self.frequency as f64 / self.sample_rate.max(1.0) as f64).fract();
    }

    fn transform(&mut self) -> Vec<Block> {
        let phase_increment = (self.frequency / self.sample_rate.max(1.0)) as f64;

        let output: Block = self
            .source
            .iter()
            .map(|frame| {
                let wave = (self.phase as f32 * std::f32::consts::TAU).sin();
                let modulation = 1.0 - self.depth * (0.5 * (1.0 + wave));
                self.phase += phase_increment;
                while self.phase >= 1.0 {
                    self.phase -= 1.0;
                }
                std::array::from_fn(|ch| frame[ch] * modulation)
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
    fn the_engine_can_inject_the_sample_rate() {
        // Without this the LFO ran at the frequency per *sample*, not per second.
        let mut filter = Tremolo::default();
        assert!(filter.supports_parameter("sample_rate"));
        assert!(filter.set_parameter("sample_rate", 48_000.0));
        assert_eq!(filter.sample_rate, 48_000.0);
    }

    #[test]
    fn a_full_period_returns_to_its_starting_depth() {
        let mut filter = Tremolo::new(1.0, 1.0, 100.0);
        filter.push(Arc::new(vec![[1.0, 1.0]; 201]), 0);
        let block = filter.transform().remove(0);
        assert!((block[0][0] - block[100][0]).abs() < 1e-5);
        assert!((block[0][0] - block[200][0]).abs() < 1e-5);
        // And it actually moves in between.
        assert!((block[0][0] - block[25][0]).abs() > 0.4);
    }
}
