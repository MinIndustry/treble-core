use std::collections::VecDeque;
use std::fmt;
use std::sync::Arc;

use treble_derive::FilterMetaData;

use crate::core::graph::{Entry, Filter};
use crate::core::{Block, CHANNELS, Frame};

/// Room scale that reproduces what `amount = 0.3` used to imply.
///
/// `amount` used to set the room size, the comb feedback and the wet mix all at
/// once; `size` and `decay` took the first two over. Their defaults are the
/// values the old single control produced at *its* default, so a reverb left
/// alone sounds exactly as it did.
const DEFAULT_SIZE: f32 = 0.975;

/// Comb feedback that reproduces what `amount = 0.3` used to imply. See
/// [`DEFAULT_SIZE`].
const DEFAULT_DECAY: f32 = 0.655;

/// Highest comb feedback the parameter permits.
///
/// Each comb is stable for any feedback below 1.0 and the damping filter has
/// unity gain at DC, so it does not move that bound; the ceiling is here
/// because the last few hundredths buy a tail measured in tens of seconds that
/// no longer sounds like a room.
const MAX_DECAY: f32 = 0.95;

/// Compact Schroeder-style ambience built from four parallel feedback combs.
///
/// `amount` is the wet/dry mix. `size` scales the comb lengths, `decay` sets
/// their feedback, and `damping` low-passes inside the feedback path — an
/// undamped comb returns every reflection with its treble intact, which is
/// what makes a cheap reverb sound like a metal tank rather than like a room.
#[derive(FilterMetaData, Clone)]
pub struct ReverbFilter {
    #[filter_source]
    source: Arc<Block>,
    /// Wet/dry mix, and nothing else. It used to scale the room and the
    /// feedback too; `size` and `decay` are those, separately.
    #[filter_parameter(range, 0.0, 1.0, 0.3)]
    amount: f32,
    /// Multiplier on the comb lengths, so the room is deeper or tighter
    /// without the tail getting louder.
    #[filter_parameter(range, 0.5, 2.0, 0.975)]
    size: f32,
    /// Comb feedback. How much of each reflection comes back, which is what
    /// sets the length of the tail.
    #[filter_parameter(range, 0.0, 0.95, 0.655)]
    decay: f32,
    /// Fraction of the previous fed-back sample each comb retains: a one-pole
    /// low-pass inside the loop, so each pass through the room loses treble
    /// the way a room with soft surfaces does. Zero is the bypass, and is the
    /// default because the filter had no damping before and a non-zero default
    /// would re-voice every existing tail.
    #[filter_parameter(range, 0.0, 0.95, 0.0)]
    damping: f32,
    #[filter_parameter(range, 1.0, 192000.0, 44100.0)]
    sample_rate: f32,
    buffers: Vec<VecDeque<Frame>>,
    /// One-pole state per comb, per channel.
    stores: [Frame; Self::COMBS],
    /// The `(size, rate)` the combs were built for. Rebuilding drops the tail,
    /// so it must not happen for a change that does not alter their length.
    configured_for: (f32, f32),
}

impl ReverbFilter {
    const COMBS: usize = 4;
    const DELAYS: [f32; Self::COMBS] = [0.0297, 0.0371, 0.0411, 0.0437];

    pub fn new(sample_rate: f32, amount: f32) -> Self {
        Self::with_room(sample_rate, amount, DEFAULT_SIZE, DEFAULT_DECAY, 0.0)
    }

    pub fn with_room(sample_rate: f32, amount: f32, size: f32, decay: f32, damping: f32) -> Self {
        let mut filter = Self {
            source: Arc::new(Vec::new()),
            amount,
            size,
            decay,
            damping,
            sample_rate,
            buffers: Vec::new(),
            stores: [[0.0; CHANNELS]; Self::COMBS],
            configured_for: (-1.0, -1.0),
        };
        filter.ensure_buffers();
        filter
    }

    fn ensure_buffers(&mut self) {
        let configuration = (self.size, self.sample_rate);
        if configuration == self.configured_for {
            return;
        }
        let room_scale = self.size.clamp(0.5, 2.0);
        self.buffers = Self::DELAYS
            .iter()
            .map(|delay| {
                let frames = (delay * room_scale * self.sample_rate).round().max(1.0) as usize;
                VecDeque::from(vec![[0.0; CHANNELS]; frames])
            })
            .collect();
        self.stores = [[0.0; CHANNELS]; Self::COMBS];
        self.configured_for = configuration;
    }
}

impl Default for ReverbFilter {
    fn default() -> Self {
        Self::new(44_100.0, 0.3)
    }
}

impl Entry for ReverbFilter {
    fn push(&mut self, block: Arc<Block>, _port: usize) {
        self.source = block;
    }
}

impl fmt::Display for ReverbFilter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Reverb Filter - {:.0}% wet, size: {}, decay: {}, damping: {}",
            self.amount * 100.0,
            self.size,
            self.decay,
            self.damping
        )
    }
}

impl fmt::Debug for ReverbFilter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReverbFilter")
            .field("amount", &self.amount)
            .field("size", &self.size)
            .field("decay", &self.decay)
            .field("damping", &self.damping)
            .field("sample_rate", &self.sample_rate)
            .finish()
    }
}

impl Filter for ReverbFilter {
    fn transform(&mut self) -> Vec<Block> {
        self.ensure_buffers();
        let amount = self.amount.clamp(0.0, 1.0);
        let feedback = self.decay.clamp(0.0, MAX_DECAY);
        let damping = self.damping.clamp(0.0, MAX_DECAY);
        let combs = Self::COMBS as f32;

        let source = std::mem::take(&mut self.source);
        let mut output = Vec::with_capacity(source.len());
        let share = 1.0 / combs;
        for input in source.iter() {
            let mut wet = [0.0; CHANNELS];
            // Zipped rather than indexed by comb: `self.stores[comb]` in here
            // measured 3.58 us per 512-frame block against 3.19 us for this,
            // and running the combs one at a time over the whole block instead
            // was worse again at 7.69 us — the running `wet` sum has to stay in
            // registers rather than become a second buffer.
            for (buffer, store) in self.buffers.iter_mut().zip(self.stores.iter_mut()) {
                let delayed = buffer.pop_front().unwrap_or([0.0; CHANNELS]);
                // The low-pass sits in the feedback path, not on the output:
                // the first reflection keeps its treble and each further pass
                // loses more, which is what absorption does. Filtering the
                // output instead would just make the whole reverb duller.
                *store = std::array::from_fn(|channel| {
                    delayed[channel] * (1.0 - damping) + store[channel] * damping
                });
                buffer.push_back(std::array::from_fn(|channel| {
                    input[channel] + store[channel] * feedback
                }));
                for channel in 0..CHANNELS {
                    wet[channel] += delayed[channel] * share;
                }
            }
            output.push(std::array::from_fn(|channel| {
                input[channel] * (1.0 - amount) + wet[channel] * amount
            }));
        }
        vec![output]
    }

    fn postponable(&self) -> bool {
        true
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use treble_meta::MetaFilter;

    const RATE: f32 = 8_000.0;
    /// Long enough at 8 kHz for the longest comb (44 ms) to come round many
    /// times.
    const TAIL: usize = 8_000;

    /// An impulse, then silence: a flat-spectrum excitation, so what comes out
    /// is the reverb's own colour.
    fn strike(filter: &mut ReverbFilter, frames: usize) -> Block {
        let mut input = vec![[0.0f32; CHANNELS]; frames];
        input[0] = [1.0; CHANNELS];
        filter.push(Arc::new(input), 0);
        filter.transform().remove(0)
    }

    fn rms(samples: &[f32]) -> f32 {
        let sum: f64 = samples.iter().map(|s| (*s as f64) * (*s as f64)).sum();
        (sum / samples.len().max(1) as f64).sqrt() as f32
    }

    /// The late tail, one channel — well past the first reflections, which
    /// carry most of the energy and are deliberately left bright.
    fn tail(block: &Block) -> Vec<f32> {
        block[3_000..].iter().map(|frame| frame[0]).collect()
    }

    /// How much of the tail's energy sits at the top of the band, as the RMS of
    /// its first difference against the RMS of the signal.
    ///
    /// A first difference is a 6 dB/octave high-pass, so the ratio falls as
    /// the content darkens. Cruder than a filter bank, but it needs no
    /// reference spectrum and cannot be fooled by the comb structure landing
    /// on or off an analysis bin.
    fn brightness(samples: &[f32]) -> f32 {
        let differences: Vec<f32> = samples.windows(2).map(|pair| pair[1] - pair[0]).collect();
        rms(&differences) / rms(samples).max(1e-12)
    }

    #[test]
    fn damping_darkens_the_tail() {
        let mut bright = ReverbFilter::with_room(RATE, 1.0, 1.0, 0.85, 0.0);
        let open = brightness(&tail(&strike(&mut bright, TAIL)));

        let mut absorbent = ReverbFilter::with_room(RATE, 1.0, 1.0, 0.85, 0.9);
        let damped = brightness(&tail(&strike(&mut absorbent, TAIL)));

        assert!(
            damped < open * 0.5,
            "damping should take most of the treble out of the tail: {damped:.4} \
             against {open:.4} undamped"
        );
    }

    #[test]
    fn damping_leaves_the_first_reflections_alone() {
        // The low-pass is inside the loop, so the earliest reflections should
        // arrive at full brightness and only later passes darken. If damping
        // were on the output the whole tail would dull evenly.
        let mut absorbent = ReverbFilter::with_room(RATE, 1.0, 1.0, 0.85, 0.9);
        let block = strike(&mut absorbent, TAIL);
        let early: Vec<f32> = block[..400].iter().map(|frame| frame[0]).collect();
        let late: Vec<f32> = block[4_000..].iter().map(|frame| frame[0]).collect();
        assert!(
            brightness(&late) < brightness(&early) * 0.7,
            "early {:.4} vs late {:.4}",
            brightness(&early),
            brightness(&late)
        );
    }

    #[test]
    fn decay_sets_the_length_of_the_tail() {
        let mut short = ReverbFilter::with_room(RATE, 1.0, 1.0, 0.3, 0.0);
        let mut long = ReverbFilter::with_room(RATE, 1.0, 1.0, 0.9, 0.0);
        let quick = rms(&strike(&mut short, TAIL)[4_000..]
            .iter()
            .map(|f| f[0])
            .collect::<Vec<_>>());
        let slow = rms(&strike(&mut long, TAIL)[4_000..]
            .iter()
            .map(|f| f[0])
            .collect::<Vec<_>>());
        assert!(
            slow > quick * 10.0,
            "a 0.9 feedback should still be ringing where 0.3 has gone: \
             {slow:.6} against {quick:.6}"
        );
    }

    #[test]
    fn amount_only_moves_the_wet_dry_balance() {
        // The point of the split: `amount` used to lengthen the tail and grow
        // the room as it was raised. Now doubling it doubles the wet signal
        // and changes nothing else, so the same room can be sent more or less
        // of the same source.
        let mut quiet = ReverbFilter::with_room(RATE, 0.25, 1.0, 0.8, 0.2);
        let mut loud = ReverbFilter::with_room(RATE, 0.5, 1.0, 0.8, 0.2);
        let one = strike(&mut quiet, 2_000);
        let two = strike(&mut loud, 2_000);
        // Past the dry impulse the output is wet only, so the ratio is exact.
        for frame in 100..2_000 {
            assert!(
                (two[frame][0] - 2.0 * one[frame][0]).abs() < 1e-5,
                "frame {frame}: {} is not twice {}",
                two[frame][0],
                one[frame][0]
            );
        }
    }

    #[test]
    fn size_scales_the_first_reflection() {
        // Half the room, half the time to the first return of the shortest
        // comb: 29.7 ms at 8 kHz is 238 frames, so 119 at size 0.5.
        let mut small = ReverbFilter::with_room(RATE, 1.0, 0.5, 0.8, 0.0);
        let block = strike(&mut small, 1_000);
        let first = block
            .iter()
            .position(|frame| frame[0].abs() > 0.1)
            .expect("the impulse comes back");
        assert!(
            (119..=121).contains(&first),
            "the first reflection arrived at frame {first}, expected ~119"
        );
    }

    #[test]
    fn the_defaults_reproduce_the_single_amount_control() {
        // `ReverbFilter::new` is what every existing caller uses, and it must
        // keep meaning what it meant: the room and feedback `amount = 0.3`
        // implied, whatever the wet mix is set to.
        let filter = ReverbFilter::new(RATE, 0.7);
        assert_eq!(filter.size, DEFAULT_SIZE);
        assert_eq!(filter.decay, DEFAULT_DECAY);
        assert_eq!(filter.damping, 0.0);
    }

    #[test]
    fn the_engine_can_inject_every_parameter_by_name() {
        let mut filter = ReverbFilter::default();
        for name in ["amount", "size", "decay", "damping", "sample_rate"] {
            assert!(
                filter.supports_parameter(name),
                "'{name}' must be settable from an FxSpec"
            );
        }
        assert!(!filter.set_parameter("nonsense", 1.0));
        // A feedback of 1.0 would ring forever; the range has to clamp it.
        assert!(filter.set_parameter("decay", 5.0));
        assert!(filter.decay <= MAX_DECAY);
        let block = strike(&mut filter, 512);
        assert!(block.iter().all(|frame| frame[0].is_finite()));
    }
}
