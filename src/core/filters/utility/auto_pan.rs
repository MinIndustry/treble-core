use std::fmt;
use std::sync::Arc;

use treble_derive::FilterMetaData;

use crate::core::Block;
use crate::core::graph::{Entry, Filter};

/// LFO shapes an [`AutoPanFilter`] can sweep with.
///
/// Stored as an ordinal because filter parameters cross the boundary as `f32`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PanWave {
    /// Smooth sweep, starting centred and moving right.
    #[default]
    Sine,
    /// Linear sweep, starting centred and moving right.
    Triangle,
    /// Hard alternation between the two sides.
    Square,
    /// Ramp from left to right, then jump back.
    Saw,
    /// One random position held for each period.
    Random,
}

impl PanWave {
    pub fn from_ordinal(ordinal: u8) -> Self {
        match ordinal {
            1 => Self::Triangle,
            2 => Self::Square,
            3 => Self::Saw,
            4 => Self::Random,
            _ => Self::Sine,
        }
    }

    /// The wave's value in `-1.0..=1.0` at normalised phase `phase` (`0.0..1.0`).
    fn value(self, phase: f64, period: u64) -> f32 {
        let phase = phase as f32;
        match self {
            Self::Sine => (phase * std::f32::consts::TAU).sin(),
            // Starts at 0 and rises, so it lines up with the sine's phase.
            Self::Triangle => {
                if phase < 0.25 {
                    4.0 * phase
                } else if phase < 0.75 {
                    2.0 - 4.0 * phase
                } else {
                    4.0 * phase - 4.0
                }
            }
            Self::Square => {
                if phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            Self::Saw => 2.0 * phase - 1.0,
            Self::Random => unit_from_index(period),
        }
    }
}

/// A deterministic value in `-1.0..=1.0` for a period index.
///
/// Hashed rather than drawn from an RNG so that a graph replays identically.
fn unit_from_index(index: u64) -> f32 {
    let mut state = index.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    state ^= state >> 30;
    state = state.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    state ^= state >> 27;
    state = state.wrapping_mul(0x94D0_49BB_1331_11EB);
    state ^= state >> 31;
    ((state >> 40) as f32 / (1u32 << 24) as f32) * 2.0 - 1.0
}

/// Sweeps the stereo position with an LFO.
///
/// The static [`PanFilter`](super::pan::PanFilter) applies one fixed position;
/// this moves it. Both use the same equal-power law, so a sweep passing
/// through the centre matches a static centred pan in level, and perceived
/// loudness stays constant across the sweep.
#[derive(FilterMetaData, Clone)]
pub struct AutoPanFilter {
    #[filter_source]
    source: Arc<Block>,
    /// Normalised phase in `0.0..1.0`. Kept at f64 because a slow sweep
    /// accumulates millions of tiny increments over a performance and f32
    /// would drift out of time.
    phase: f64,
    /// Completed periods, used to step the random shape.
    period: u64,
    #[filter_parameter(range, 0.0, 100.0, 0.5)]
    frequency: f32,
    #[filter_parameter(range, 0.0, 1.0, 1.0)]
    depth: f32,
    #[filter_parameter(int, 0, 0, 4)]
    waveform: u8,
    // Registered so the engine injects its real rate at build time; a plain
    // field would silently keep the default and mistune the sweep.
    #[filter_parameter(range, 1.0, 192000.0, 44100.0)]
    sample_rate: f32,
}

impl AutoPanFilter {
    pub fn new(frequency: f32, depth: f32, waveform: PanWave, sample_rate: f32) -> Self {
        Self {
            source: Arc::new(Vec::new()),
            phase: 0.0,
            period: 0,
            frequency,
            depth,
            waveform: waveform as u8,
            sample_rate,
        }
    }

    pub fn wave(&self) -> PanWave {
        PanWave::from_ordinal(self.waveform)
    }
}

impl Default for AutoPanFilter {
    fn default() -> Self {
        Self::new(0.5, 1.0, PanWave::Sine, 44100.0)
    }
}

impl Entry for AutoPanFilter {
    fn push(&mut self, block: Arc<Block>, _port: usize) {
        self.source = block;
    }
}

impl fmt::Display for AutoPanFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Auto Pan Filter - {:?} {}Hz, depth: {}",
            self.wave(),
            self.frequency,
            self.depth
        )
    }
}

impl fmt::Debug for AutoPanFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AutoPanFilter {{ waveform: {:?}, frequency: {}, depth: {} }}",
            self.wave(),
            self.frequency,
            self.depth
        )
    }
}

impl Filter for AutoPanFilter {
    /// Phase is a function of the timeline, not of instance age: a replacement
    /// filter created by a mid-performance edit resumes the sweep exactly
    /// where the old one was, and same-rate sweeps on different patterns stay
    /// aligned. Within a block the per-sample increment continues from here.
    fn on_transport(&mut self, frame: u64) {
        let cycles = frame as f64 * self.frequency as f64 / self.sample_rate.max(1.0) as f64;
        self.phase = cycles.fract();
        self.period = cycles as u64;
    }

    fn transform(&mut self) -> Vec<Block> {
        let wave = self.wave();
        let increment = (self.frequency / self.sample_rate.max(1.0)) as f64;
        let depth = self.depth.clamp(0.0, 1.0);

        let output: Block = self
            .source
            .iter()
            .map(|[left, right]| {
                let direction = (depth * wave.value(self.phase, self.period)).clamp(-1.0, 1.0);
                // Equal-power law, matching PanFilter.
                let theta = (direction + 1.0) * std::f32::consts::FRAC_PI_4;
                let (right_gain, left_gain) = theta.sin_cos();
                self.phase += increment;
                while self.phase >= 1.0 {
                    self.phase -= 1.0;
                    self.period = self.period.wrapping_add(1);
                }
                [*left * left_gain, *right * right_gain]
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

    fn run(filter: &mut AutoPanFilter, frames: usize) -> Block {
        filter.push(Arc::new(vec![[1.0, 1.0]; frames]), 0);
        filter.transform().remove(0)
    }

    #[test]
    fn a_full_period_sweeps_both_sides_and_returns() {
        // One period over exactly 100 frames.
        let mut filter = AutoPanFilter::new(1.0, 1.0, PanWave::Sine, 100.0);
        let block = run(&mut filter, 100);

        // Starts centred (equal-power: sqrt(2)/2 per side), peaks right at a
        // quarter, peaks left at three.
        let centre = std::f32::consts::FRAC_1_SQRT_2;
        assert!((block[0][0] - centre).abs() < 1e-6);
        assert!((block[0][1] - centre).abs() < 1e-6);
        assert!(block[25][1] > 0.99 && block[25][0] < 0.01);
        assert!(block[75][0] > 0.99 && block[75][1] < 0.01);
    }

    #[test]
    fn the_sweep_stays_in_time_over_many_periods() {
        let mut filter = AutoPanFilter::new(1.0, 1.0, PanWave::Sine, 100.0);
        // 500 periods is far enough for f32 phase accumulation to drift.
        let block = run(&mut filter, 100 * 500);
        let centre = std::f32::consts::FRAC_1_SQRT_2;
        for period in 0..500 {
            let frame = block[period * 100];
            assert!(
                (frame[0] - centre).abs() < 1e-4 && (frame[1] - centre).abs() < 1e-4,
                "period {period} started at {frame:?} instead of centred"
            );
        }
        let quarter = block[100 * 499 + 25];
        assert!(
            quarter[1] > 0.99 && quarter[0] < 0.01,
            "the last period peaked at {quarter:?}"
        );
    }

    #[test]
    fn depth_narrows_the_sweep() {
        let mut filter = AutoPanFilter::new(1.0, 0.5, PanWave::Sine, 100.0);
        let block = run(&mut filter, 100);
        let widest = block
            .iter()
            .map(|frame| (frame[1] - frame[0]).abs())
            .fold(0.0f32, f32::max);
        // Half depth peaks at direction 0.5. Equal-power puts the channel
        // difference there at sin(3pi/8) - cos(3pi/8).
        let theta = 1.5 * std::f32::consts::FRAC_PI_4;
        let expected = theta.sin() - theta.cos();
        assert!(
            (widest - expected).abs() < 1e-3,
            "widest swing was {widest}, expected {expected}"
        );
    }

    #[test]
    fn zero_depth_matches_a_centred_static_pan() {
        let mut filter = AutoPanFilter::new(4.0, 0.0, PanWave::Sine, 100.0);
        let block = run(&mut filter, 32);
        let centre = std::f32::consts::FRAC_1_SQRT_2;
        assert!(
            block
                .iter()
                .all(|frame| (frame[0] - centre).abs() < 1e-6 && (frame[1] - centre).abs() < 1e-6),
            "a zero-depth sweep must sit still at the centre"
        );
    }

    #[test]
    fn square_alternates_hard_between_the_sides() {
        let mut filter = AutoPanFilter::new(1.0, 1.0, PanWave::Square, 100.0);
        let block = run(&mut filter, 100);
        assert!(block[10][1] > 0.99 && block[10][0] < 0.01);
        assert!(block[60][0] > 0.99 && block[60][1] < 0.01);
    }

    #[test]
    fn random_holds_one_position_per_period_and_repeats() {
        let mut first = AutoPanFilter::new(1.0, 1.0, PanWave::Random, 100.0);
        let block = run(&mut first, 250);
        // Held for the whole period...
        assert!((block[10][0] - block[80][0]).abs() < 1e-6);
        // ...then it moves.
        assert!((block[10][0] - block[110][0]).abs() > 1e-6);

        // Same filter, same stream: identical output.
        let mut second = AutoPanFilter::new(1.0, 1.0, PanWave::Random, 100.0);
        let repeat = run(&mut second, 250);
        assert_eq!(block, repeat);
    }

    /// The reported bug: editing a swept pan's depth mid-song hot-swaps in a
    /// fresh filter, which used to restart the sweep from phase zero.
    #[test]
    fn a_hot_swapped_replacement_resumes_the_sweep_phase() {
        // One sweep per second at 100 Hz; run the "old" filter to frame 37,
        // an arbitrary point mid-sweep.
        let mut old = AutoPanFilter::new(1.0, 1.0, PanWave::Sine, 100.0);
        old.on_transport(0);
        let _ = run(&mut old, 37);
        let old_continuation = run(&mut old, 20);

        // The edit builds a brand-new instance; the render thread anchors it
        // to the same timeline before its first block.
        let mut new = AutoPanFilter::new(1.0, 1.0, PanWave::Sine, 100.0);
        new.on_transport(37);
        let new_output = run(&mut new, 20);

        for (frame, (a, b)) in old_continuation.iter().zip(new_output.iter()).enumerate() {
            assert!(
                (a[0] - b[0]).abs() < 1e-5 && (a[1] - b[1]).abs() < 1e-5,
                "frame {frame}: old {a:?} vs new {b:?} — the sweep restarted"
            );
        }
    }

    #[test]
    fn a_depth_edit_keeps_the_position_moving_from_where_it_was() {
        // Depth is the parameter from the report: the swap changes depth but
        // must not reset where the LFO is in its cycle.
        let mut old = AutoPanFilter::new(1.0, 1.0, PanWave::Sine, 100.0);
        old.on_transport(0);
        let _ = run(&mut old, 25); // quarter sweep: hard right

        let mut edited = AutoPanFilter::new(1.0, 0.5, PanWave::Sine, 100.0);
        edited.on_transport(25);
        let block = run(&mut edited, 1);
        // Still at the quarter point — direction 0.5 (depth-scaled), not back
        // at the centre. Equal-power at direction 0.5:
        let theta = 1.5 * std::f32::consts::FRAC_PI_4;
        assert!(
            (block[0][0] - theta.cos()).abs() < 1e-3 && (block[0][1] - theta.sin()).abs() < 1e-3,
            "expected the quarter-sweep position at reduced depth, got {:?}",
            block[0]
        );
    }

    #[test]
    fn the_random_wave_period_is_timeline_anchored_too() {
        let mut old = AutoPanFilter::new(1.0, 1.0, PanWave::Random, 100.0);
        old.on_transport(0);
        let stream = run(&mut old, 250);

        // A replacement anchored mid-way holds the same random position.
        let mut new = AutoPanFilter::new(1.0, 1.0, PanWave::Random, 100.0);
        new.on_transport(150);
        let resumed = run(&mut new, 10);
        assert!(
            (stream[150][0] - resumed[0][0]).abs() < 1e-6,
            "random period did not resume: {} vs {}",
            stream[150][0],
            resumed[0][0]
        );
    }

    #[test]
    fn the_engine_can_inject_every_parameter_by_name() {
        let mut filter = AutoPanFilter::default();
        for name in ["frequency", "depth", "waveform", "sample_rate"] {
            assert!(
                filter.supports_parameter(name),
                "'{name}' must be settable from an FxSpec"
            );
        }
        assert!(filter.set_parameter("waveform", 2.0));
        assert_eq!(filter.wave(), PanWave::Square);
        // Out-of-range values clamp rather than corrupt the sweep.
        assert!(filter.set_parameter("waveform", 99.0));
        assert_eq!(filter.wave(), PanWave::Random);
        assert!(filter.set_parameter("depth", -3.0));
        assert!(!filter.set_parameter("nonsense", 1.0));
    }
}
