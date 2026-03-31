use std::sync::Arc;

use crate::core::graph::{Entry, Sink};
use crate::core::{Block, CHANNELS, Frame};

const DEFAULT_LIMITER_THRESHOLD: f32 = 0.95;
const DEFAULT_LIMITER_ATTACK: f32 = 0.001;
const DEFAULT_LIMITER_RELEASE: f32 = 0.2;

/// The final output sink. Owns three responsibilities:
///
/// 1. **Mixing** — sums all connected instrument streams. Each `push()` call
///    accumulates its block into a per-cycle sum buffer (first push initialises,
///    subsequent ones add on top).
///
/// 2. **Master volume** — linear gain applied before limiting so the limiter
///    always acts as a hard ceiling regardless of the volume setting.
///
/// 3. **Peak-tracking limiter** — brick-wall limiter applied in `consume()` after
///    the sum and gain are finalised. Gain formula: `threshold / envelope` when
///    the envelope exceeds threshold (infinite ratio).
///
/// Parameters settable via [`Sink::set_parameter`]:
/// - `"master_volume"` — linear output gain (default 1.0)
/// - `"limiter_threshold"` — ceiling in linear amplitude (default 0.95)
/// - `"limiter_attack"` — attack time in seconds (default 0.001)
/// - `"limiter_release"` — release time in seconds (default 0.2)
/// - `"sample_rate"` — sample rate for limiter coefficients (default 44100.0)
#[derive(Clone, Debug)]
pub struct AudioOutputSink {
    /// Per-cycle accumulation buffer — reset after each `consume()`.
    accumulator: Vec<Frame>,
    master_volume: f32,
    limiter_threshold: f32,
    limiter_attack: f32,
    limiter_release: f32,
    /// Per-channel peak envelope state (carries over between blocks).
    limiter_envelope: [f32; CHANNELS],
    sample_rate: f32,
}

impl AudioOutputSink {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            accumulator: Vec::new(),
            master_volume: 1.0,
            limiter_threshold: DEFAULT_LIMITER_THRESHOLD,
            limiter_attack: DEFAULT_LIMITER_ATTACK,
            limiter_release: DEFAULT_LIMITER_RELEASE,
            limiter_envelope: [0.0; CHANNELS],
            sample_rate,
        }
    }
}

impl Default for AudioOutputSink {
    fn default() -> Self {
        Self::new(44100.0)
    }
}

impl Entry for AudioOutputSink {
    /// Accumulate an incoming block into the per-cycle sum buffer.
    /// The first push for a cycle initialises the buffer; subsequent pushes sum on top.
    fn push(&mut self, block: Arc<Block>, _port: usize) {
        if self.accumulator.is_empty() {
            self.accumulator.extend(block.iter().copied());
        } else {
            for (acc, frame) in self.accumulator.iter_mut().zip(block.iter()) {
                for ch in 0..CHANNELS {
                    acc[ch] += frame[ch];
                }
            }
        }
    }
}

impl Sink for AudioOutputSink {
    /// Apply master_volume → limiter to the accumulated sum, then clear the buffer.
    fn consume(&mut self) -> Block {
        let attack_coeff = (-1.0 / (self.limiter_attack * self.sample_rate)).exp();
        let release_coeff = (-1.0 / (self.limiter_release * self.sample_rate)).exp();

        // iter() not par_iter(): limiter envelope is frame-sequential (each frame
        // depends on the previous frame's envelope value).
        let output: Block = self
            .accumulator
            .iter()
            .map(|frame| {
                std::array::from_fn(|ch| {
                    let sample = frame[ch] * self.master_volume;
                    let input_abs = sample.abs();
                    if input_abs > self.limiter_envelope[ch] {
                        self.limiter_envelope[ch] =
                            attack_coeff * (self.limiter_envelope[ch] - input_abs) + input_abs;
                    } else {
                        self.limiter_envelope[ch] =
                            release_coeff * (self.limiter_envelope[ch] - input_abs) + input_abs;
                    }
                    let gain = if self.limiter_envelope[ch] > self.limiter_threshold {
                        self.limiter_threshold / self.limiter_envelope[ch]
                    } else {
                        1.0
                    };
                    sample * gain
                })
            })
            .collect();

        self.accumulator.clear();
        output
    }

    fn get_frames(&self) -> &[Frame] {
        &self.accumulator
    }

    fn into_entry(self) -> Box<dyn Entry> {
        Box::new(self)
    }

    fn set_parameter(&mut self, name: &str, value: f32) {
        match name {
            "master_volume" => self.master_volume = value,
            "limiter_threshold" => self.limiter_threshold = value,
            "limiter_attack" => self.limiter_attack = value,
            "limiter_release" => self.limiter_release = value,
            "sample_rate" => self.sample_rate = value,
            _ => {}
        }
    }
}
