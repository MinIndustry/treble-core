use std::fmt;
use std::sync::Arc;

use treble_derive::FilterMetaData;

use crate::core::graph::{Entry, Filter};
use crate::core::{Block, CHANNELS, Frame};

/// Shortest base delay the sweep is allowed to sit at, in milliseconds.
///
/// Below roughly 5 ms the comb notches move up into the band the ear reads as
/// timbre rather than as space, and the effect stops being a chorus and starts
/// being a flanger.
const MIN_DELAY_MS: f32 = 5.0;

/// Longest base delay, in milliseconds. Past ~30 ms the copy is heard as a
/// separate slap rather than fused with the dry signal.
const MAX_DELAY_MS: f32 = 30.0;

/// Fraction of the base delay the LFO swings either side of it at `depth = 1`.
///
/// The delay's rate of change *is* the detune: a sine sweep of `a` seconds
/// amplitude at `f` Hz peaks at `2*PI*f*a` in pitch ratio. At the defaults
/// that is about 26 cents, and at full depth and the top of the rate range it
/// is far more than any chorus wants — which is the point of having a depth
/// control rather than a fixed excursion.
const MAX_EXCURSION: f32 = 0.5;

/// LFO phase offset, in cycles, between the first and last channel at
/// `spread = 1`.
///
/// Half a cycle is full anti-phase: when one channel is at its longest delay
/// the other is at its shortest, which is the widest a two-channel chorus
/// gets. The default sits at half of that — a quarter cycle, the classic
/// quadrature spread — because full anti-phase partially cancels when a
/// listener collapses the mix to mono.
const MAX_SPREAD_CYCLES: f64 = 0.5;

/// Extra frames kept in the ring beyond the longest delay, so the
/// interpolator's second tap always has somewhere to land.
const INTERPOLATION_HEADROOM: usize = 2;

/// A short LFO-modulated delay: one detuned copy of the input, swept slowly
/// under the dry signal, with the two channels' LFOs offset so the result
/// widens rather than just wobbles.
#[derive(FilterMetaData, Clone)]
pub struct Chorus {
    #[filter_source]
    source: Arc<Block>,
    /// Normalised LFO phase in `0.0..1.0`. Kept at f64 because a 0.05 Hz
    /// sweep accumulates millions of tiny increments over a performance and
    /// f32 would drift out of time, as in `AutoPanFilter`.
    phase: f64,
    #[filter_parameter(range, 0.05, 8.0, 0.8)]
    frequency: f32,
    /// Centre of the sweep, in milliseconds.
    #[filter_parameter(range, 5.0, 30.0, 12.0)]
    delay: f32,
    #[filter_parameter(range, 0.0, 1.0, 0.5)]
    depth: f32,
    #[filter_parameter(range, 0.0, 1.0, 0.5)]
    spread: f32,
    /// Equal parts dry and wet by default: that is where a comb filter's
    /// notches are deepest, so the effect is unmistakable at defaults.
    #[filter_parameter(range, 0.0, 1.0, 0.5)]
    mix: f32,
    // Registered so the engine injects its real rate at build time; a plain
    // field would keep the default and both mistune the LFO and misread the
    // delay in milliseconds.
    #[filter_parameter(range, 1.0, 192000.0, 44100.0)]
    sample_rate: f32,
    /// Ring of recent input frames. Written at `write`, read behind it.
    buffer: Vec<Frame>,
    write: usize,
    /// The rate the ring was sized for. Sized for the *longest* delay the
    /// parameters permit rather than for the current one, so automating
    /// `delay` never reallocates and never drops the line's contents — a
    /// swept delay that cleared its buffer would click on every block.
    sized_for: f32,
}

impl Chorus {
    pub fn new(
        frequency: f32,
        delay: f32,
        depth: f32,
        spread: f32,
        mix: f32,
        sample_rate: f32,
    ) -> Self {
        let mut filter = Self {
            source: Arc::new(Vec::new()),
            phase: 0.0,
            frequency,
            delay,
            depth,
            spread,
            mix,
            sample_rate,
            buffer: Vec::new(),
            write: 0,
            sized_for: -1.0,
        };
        filter.ensure_buffer();
        filter
    }

    fn ensure_buffer(&mut self) {
        if self.sized_for == self.sample_rate && !self.buffer.is_empty() {
            return;
        }
        let longest = MAX_DELAY_MS * (1.0 + MAX_EXCURSION) * 0.001 * self.sample_rate.max(1.0);
        let frames = longest.ceil().max(1.0) as usize + INTERPOLATION_HEADROOM;
        self.buffer = vec![[0.0; CHANNELS]; frames];
        self.write = 0;
        self.sized_for = self.sample_rate;
    }

    /// Reads one channel `delay` frames behind the write head, interpolating
    /// between the two frames the fractional position falls between.
    ///
    /// Snapping to whole samples instead would quantise the sweep to the
    /// sample grid: the delay would step rather than glide, and each step is a
    /// discontinuity in the copy's phase. That is heard as grit riding on the
    /// sweep, and it is the usual reason a first chorus sounds broken.
    #[inline]
    fn read(&self, channel: usize, delay: f32) -> f32 {
        let length = self.buffer.len();
        let whole = delay.floor();
        let fraction = delay - whole;
        let nearer = (self.write + length - whole as usize) % length;
        let further = (nearer + length - 1) % length;
        self.buffer[nearer][channel] * (1.0 - fraction) + self.buffer[further][channel] * fraction
    }
}

impl Default for Chorus {
    fn default() -> Self {
        Self::new(0.8, 12.0, 0.5, 0.5, 0.5, 44100.0)
    }
}

impl Entry for Chorus {
    fn push(&mut self, block: Arc<Block>, _port: usize) {
        self.source = block;
    }
}

impl fmt::Display for Chorus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Chorus - {}Hz, {}ms, depth: {}, spread: {}",
            self.frequency, self.delay, self.depth, self.spread
        )
    }
}

impl fmt::Debug for Chorus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Chorus")
            .field("frequency", &self.frequency)
            .field("delay", &self.delay)
            .field("depth", &self.depth)
            .field("spread", &self.spread)
            .field("mix", &self.mix)
            .finish()
    }
}

impl Filter for Chorus {
    /// Phase is a function of the timeline, not of instance age. Editing any
    /// parameter hot-swaps the filter for a freshly built one, and a fresh
    /// instance that started its LFO at zero would jump the sweep back to the
    /// beginning — a bug reported twice against this engine's other LFOs.
    /// Deriving the phase from the engine frame here is what makes a
    /// mid-performance edit inaudible; copy this, not a self-incrementing
    /// counter.
    fn on_transport(&mut self, frame: u64) {
        self.phase =
            (frame as f64 * self.frequency as f64 / self.sample_rate.max(1.0) as f64).fract();
    }

    fn transform(&mut self) -> Vec<Block> {
        self.ensure_buffer();

        let rate = self.sample_rate.max(1.0);
        let increment = (self.frequency.max(0.0) / rate) as f64;
        let centre = self.delay.clamp(MIN_DELAY_MS, MAX_DELAY_MS) * 0.001 * rate;
        let excursion = centre * MAX_EXCURSION * self.depth.clamp(0.0, 1.0);
        let mix = self.mix.clamp(0.0, 1.0);
        // The ring's last frame is the interpolator's second tap, so the read
        // head may never reach it.
        let longest = (self.buffer.len() - INTERPOLATION_HEADROOM) as f32;

        let span = (CHANNELS.max(2) - 1) as f64;
        let spread = self.spread.clamp(0.0, 1.0) as f64 * MAX_SPREAD_CYCLES;
        let offsets: [f64; CHANNELS] =
            std::array::from_fn(|channel| spread * channel as f64 / span);

        // Taken rather than borrowed: the ring is written inside the loop, and
        // leaving a stale block in place would re-chorus it if the graph ran a
        // cycle without feeding this node.
        let source = std::mem::take(&mut self.source);
        let mut output = Vec::with_capacity(source.len());
        for input in source.iter() {
            self.buffer[self.write] = *input;
            output.push(std::array::from_fn(|channel| {
                let phase = (self.phase + offsets[channel]).fract() as f32;
                let sweep = (phase * std::f32::consts::TAU).sin();
                let delay = (centre + excursion * sweep).clamp(1.0, longest);
                let wet = self.read(channel, delay);
                input[channel] * (1.0 - mix) + wet * mix
            }));

            self.write = (self.write + 1) % self.buffer.len();
            self.phase += increment;
            while self.phase >= 1.0 {
                self.phase -= 1.0;
            }
        }

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

    /// Every test runs at 1 kHz so a millisecond of delay is one frame and the
    /// expected sweep can be written down in frames.
    const RATE: f32 = 1000.0;

    fn run(filter: &mut Chorus, block: Block) -> Block {
        filter.push(Arc::new(block), 0);
        filter.transform().remove(0)
    }

    /// An impulse every `spacing` frames — wide enough apart that each one's
    /// delayed copy lands before the next one's does.
    fn clicks(frames: usize, spacing: usize) -> Block {
        (0..frames)
            .map(|index| {
                if index % spacing == 0 {
                    [1.0; CHANNELS]
                } else {
                    [0.0; CHANNELS]
                }
            })
            .collect()
    }

    /// Where the copy of the impulse at `entered` came out, in fractional
    /// frames after it went in.
    ///
    /// The energy centroid rather than the peak: linear interpolation splits
    /// one impulse across two output frames, and the split is exactly the
    /// information this is trying to recover.
    fn measured_delay(output: &Block, channel: usize, entered: usize, window: usize) -> f32 {
        let mut weight = 0.0f64;
        let mut moment = 0.0f64;
        for offset in 0..window {
            let value = output[entered + offset][channel].abs() as f64;
            weight += value;
            moment += value * offset as f64;
        }
        assert!(weight > 0.1, "no copy of the impulse at {entered} came out");
        (moment / weight) as f32
    }

    #[test]
    fn the_delay_sweeps_across_the_lfo_cycle() {
        // One sweep per 1000 frames, centred on 12 frames, full depth: the
        // read head should travel 6..18 frames.
        let mut filter = Chorus::new(1.0, 12.0, 1.0, 0.0, 1.0, RATE);
        filter.on_transport(0);
        let output = run(&mut filter, clicks(1000, 50));

        let delays: Vec<f32> = (0..19)
            .map(|click| measured_delay(&output, 0, click * 50, 40))
            .collect();
        let shortest = delays.iter().copied().fold(f32::INFINITY, f32::min);
        let longest = delays.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        assert!(
            shortest < 7.0 && longest > 17.0,
            "the sweep only covered {shortest:.2}..{longest:.2} frames of the expected 6..18"
        );
        // A quarter of the way in the sine is at its peak, three quarters in
        // at its trough.
        assert!(
            (delays[5] - 18.0).abs() < 0.5,
            "a quarter into the cycle the delay was {:.2}, expected ~18",
            delays[5]
        );
        assert!(
            (delays[15] - 6.0).abs() < 0.5,
            "three quarters in the delay was {:.2}, expected ~6",
            delays[15]
        );
    }

    #[test]
    fn a_fractional_delay_is_interpolated_rather_than_snapped() {
        // Depth zero freezes the sweep, so the delay is exactly 12.5 frames
        // and the interpolator has nowhere to hide.
        let mut half_way = Chorus::new(1.0, 12.5, 0.0, 0.0, 1.0, RATE);
        let mut impulse: Block = vec![[0.0; CHANNELS]; 32];
        impulse[0] = [1.0; CHANNELS];
        let output = run(&mut half_way, impulse.clone());
        assert!(
            (output[12][0] - 0.5).abs() < 1e-5 && (output[13][0] - 0.5).abs() < 1e-5,
            "a 12.5-frame delay must split the impulse evenly across frames 12 \
             and 13, got {} and {}",
            output[12][0],
            output[13][0]
        );

        // And a whole-frame delay does not split it, so the halves above are
        // interpolation and not smearing.
        let mut whole = Chorus::new(1.0, 12.0, 0.0, 0.0, 1.0, RATE);
        let output = run(&mut whole, impulse);
        assert!((output[12][0] - 1.0).abs() < 1e-5);
        assert!(output[13][0].abs() < 1e-5);
    }

    #[test]
    fn the_spread_offsets_the_two_channels_sweeps() {
        let mut wide = Chorus::new(1.0, 12.0, 1.0, 1.0, 1.0, RATE);
        wide.on_transport(0);
        let output = run(&mut wide, clicks(1000, 50));
        // Full spread is anti-phase: a quarter into the cycle the left channel
        // is at its longest delay and the right at its shortest.
        let left = measured_delay(&output, 0, 250, 40);
        let right = measured_delay(&output, 1, 250, 40);
        assert!(
            (left - right).abs() > 8.0,
            "anti-phase channels should be far apart, got {left:.2} and {right:.2}"
        );

        let mut narrow = Chorus::new(1.0, 12.0, 1.0, 0.0, 1.0, RATE);
        narrow.on_transport(0);
        let output = run(&mut narrow, clicks(1000, 50));
        let left = measured_delay(&output, 0, 250, 40);
        let right = measured_delay(&output, 1, 250, 40);
        assert!(
            (left - right).abs() < 1e-4,
            "zero spread must leave the channels identical, got {left} and {right}"
        );
    }

    #[test]
    fn the_default_spread_still_separates_the_channels() {
        // A chorus whose default settings put both channels on the same LFO
        // would be wide only once somebody went looking for the control.
        let mut filter = Chorus::default();
        assert!(filter.set_parameter("sample_rate", RATE));
        assert!(filter.set_parameter("frequency", 1.0));
        // Fully wet, so the centroid measures the wet delay rather than an
        // average of the dry impulse and its copy. Everything the test is
        // about — spread and depth — is left at its default.
        assert!(filter.set_parameter("mix", 1.0));
        filter.on_transport(0);
        let output = run(&mut filter, clicks(1000, 50));
        // The default quarter-cycle offset puts the left channel at its
        // longest delay (12 + 3 frames) where the right sits at the centre.
        let left = measured_delay(&output, 0, 250, 40);
        let right = measured_delay(&output, 1, 250, 40);
        assert!(
            (left - right).abs() > 2.0,
            "the default spread barely moved: {left:.2} vs {right:.2}"
        );
    }

    /// The reported bug, twice over: editing a parameter mid-performance
    /// hot-swaps the filter, and an instance-age LFO restarts its sweep.
    #[test]
    fn a_hot_swapped_replacement_resumes_the_sweep_phase() {
        let build = || Chorus::new(1.0, 12.0, 1.0, 0.0, 1.0, RATE);
        let warmup = clicks(250, 50);
        let test = clicks(200, 50);

        // The incumbent: anchored once, then swept by its own increment.
        let mut incumbent = build();
        incumbent.on_transport(0);
        let _ = run(&mut incumbent, warmup.clone());
        let continued = run(&mut incumbent, test.clone());

        // The replacement: same buffer history, but anchored to frame 250 as
        // the render thread does before its first block.
        let mut replacement = build();
        replacement.on_transport(0);
        let _ = run(&mut replacement, warmup.clone());
        replacement.on_transport(250);
        let resumed = run(&mut replacement, test.clone());

        for (frame, (a, b)) in continued.iter().zip(resumed.iter()).enumerate() {
            assert!(
                (a[0] - b[0]).abs() < 1e-4,
                "frame {frame}: {} vs {} — the sweep did not resume",
                a[0],
                b[0]
            );
        }

        // And the same instance re-anchored to zero *does* jump, so the check
        // above is not vacuous.
        let mut restarted = build();
        restarted.on_transport(0);
        let _ = run(&mut restarted, warmup);
        restarted.on_transport(0);
        let jumped = run(&mut restarted, test);
        let difference = continued
            .iter()
            .zip(jumped.iter())
            .map(|(a, b)| (a[0] - b[0]).abs())
            .fold(0.0f32, f32::max);
        assert!(
            difference > 0.2,
            "restarting the sweep should be audible, largest difference was {difference}"
        );
    }

    #[test]
    fn sweeping_the_delay_parameter_keeps_the_line_intact() {
        // Block-rate automation calls `set_parameter` on the live filter. If
        // that resized the ring, every step of a swept delay would drop the
        // line and click.
        let mut filter = Chorus::new(1.0, 12.0, 0.0, 0.0, 1.0, RATE);
        let mut impulse: Block = vec![[0.0; CHANNELS]; 8];
        impulse[0] = [1.0; CHANNELS];
        let _ = run(&mut filter, impulse);

        assert!(filter.set_parameter("delay", 25.0));
        let tail = run(&mut filter, vec![[0.0; CHANNELS]; 32]);
        // The impulse went in at frame 0 of the previous block, so it is 8
        // frames old; a 25-frame delay puts it 17 frames into this block.
        assert!(
            (tail[17][0] - 1.0).abs() < 1e-4,
            "the buffered impulse should still be there, frame 17 was {}",
            tail[17][0]
        );
    }

    #[test]
    fn zero_mix_is_a_bypass() {
        let mut filter = Chorus::default();
        let input = clicks(64, 7);
        let assertion = input.clone();
        assert!(filter.set_parameter("mix", 0.0));
        let output = run(&mut filter, input);
        for (frame, (a, b)) in assertion.iter().zip(output.iter()).enumerate() {
            assert!((a[0] - b[0]).abs() < 1e-6, "frame {frame} was altered");
        }
    }

    #[test]
    fn an_absurd_sample_rate_stays_bounded() {
        // A spec may hold a 30 ms delay while the graph runs at 1 Hz; the read
        // head then has nowhere legal to go and must clamp rather than index
        // out of the ring.
        let mut filter = Chorus::new(8.0, 30.0, 1.0, 1.0, 1.0, 1.0);
        let output = run(&mut filter, clicks(64, 3));
        assert!(output.iter().all(|frame| frame[0].is_finite()));
    }

    #[test]
    fn the_engine_can_inject_every_parameter_by_name() {
        let mut filter = Chorus::default();
        for name in [
            "frequency",
            "delay",
            "depth",
            "spread",
            "mix",
            "sample_rate",
        ] {
            assert!(
                filter.supports_parameter(name),
                "'{name}' must be settable from an FxSpec"
            );
        }
        assert!(!filter.set_parameter("nonsense", 1.0));
        // Out-of-range values clamp rather than corrupt the sweep.
        assert!(filter.set_parameter("depth", 40.0));
        assert!(filter.set_parameter("delay", 0.0));
        let output = run(&mut filter, clicks(64, 7));
        assert!(output.iter().all(|frame| frame[0].is_finite()));
    }
}
