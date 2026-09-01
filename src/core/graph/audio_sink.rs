use std::sync::Arc;

use crate::core::graph::{Entry, Sink, SinkTelemetry};
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
///    the sum and gain are finalised.
///
///    The gain is smoothed for musicality but is never allowed above what the
///    *current sample* needs, so no sample can leave above the threshold. That
///    guarantee is the whole point of a ceiling: deriving the gain from a
///    lagging envelope alone let roughly 10 ms of every transient through at up
///    to twice the threshold, which the output stage then hard-clipped.
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
    /// Per-channel smoothed gain, carried between blocks. Starts at unity —
    /// no reduction until something asks for it.
    limiter_gain: [f32; CHANNELS],
    sample_rate: f32,
    telemetry: SinkTelemetry,
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
            limiter_gain: [1.0; CHANNELS],
            sample_rate,
            telemetry: SinkTelemetry::default(),
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
        let mut output = Block::with_capacity(self.accumulator.len());
        let mut telemetry = SinkTelemetry::default();
        let mut minimum_gain = 1.0f32;
        for frame in &self.accumulator {
            let rendered = std::array::from_fn(|ch| {
                let sample = frame[ch] * self.master_volume;
                let input_abs = sample.abs();
                telemetry.pre_limiter_peak = telemetry.pre_limiter_peak.max(input_abs);

                // What this sample needs right now to sit under the ceiling.
                let required = if input_abs > self.limiter_threshold {
                    self.limiter_threshold / input_abs
                } else {
                    1.0
                };

                // The envelope still tracks the peak, so the release stays
                // programme-dependent rather than per-sample twitchy.
                self.limiter_envelope[ch] = input_abs
                    .max(release_coeff * (self.limiter_envelope[ch] - input_abs) + input_abs);

                // Smooth the reduction going down (attack) and the recovery
                // coming back up (release), so a single transient does not
                // step the whole mix.
                let coeff = if required < self.limiter_gain[ch] {
                    attack_coeff
                } else {
                    release_coeff
                };
                self.limiter_gain[ch] = coeff * (self.limiter_gain[ch] - required) + required;

                // The smoothing may lag; `required` is the hard guarantee.
                // Taking the smaller of the two is what makes this a
                // ceiling rather than a suggestion.
                let applied_gain = self.limiter_gain[ch].min(required);
                if applied_gain < 1.0 - f32::EPSILON {
                    telemetry.limited_samples += 1;
                }
                minimum_gain = minimum_gain.min(applied_gain);
                let limited = sample * applied_gain;
                telemetry.post_limiter_peak = telemetry.post_limiter_peak.max(limited.abs());
                limited
            });
            output.push(rendered);
        }
        telemetry.max_gain_reduction_db = if minimum_gain > 0.0 {
            -20.0 * minimum_gain.log10()
        } else {
            f32::INFINITY
        };
        self.telemetry = telemetry;

        self.accumulator.clear();
        output
    }

    fn get_frames(&self) -> &[Frame] {
        &self.accumulator
    }

    fn into_entry(self) -> Box<dyn Entry> {
        Box::new(self)
    }

    fn set_parameter(&mut self, name: &str, value: f32) -> bool {
        match name {
            "master_volume" => self.master_volume = value,
            "limiter_threshold" => self.limiter_threshold = value,
            "limiter_attack" => self.limiter_attack = value,
            "limiter_release" => self.limiter_release = value,
            "sample_rate" => self.sample_rate = value,
            _ => return false,
        }
        true
    }

    fn telemetry(&self) -> Option<SinkTelemetry> {
        Some(self.telemetry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn feed(sink: &mut AudioOutputSink, samples: &[f32]) -> Vec<f32> {
        let block: Block = samples.iter().map(|&v| [v, v]).collect();
        sink.push(Arc::new(block), 0);
        sink.consume().iter().map(|f| f[0]).collect()
    }

    /// The ceiling holds from the very first sample.
    ///
    /// It did not: the gain came from an envelope that needed roughly ten
    /// attack times to catch a transient, so a step to 2.0 left the sink at
    /// 2.0 — more than twice the ceiling — and the output stage hard-clipped
    /// whatever came out above 1.0. Every note onset in a render did this.
    #[test]
    fn nothing_escapes_the_ceiling() {
        let mut sink = AudioOutputSink::new(44100.0);
        let out = feed(&mut sink, &[2.0; 512]);
        let worst = out.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(
            worst <= DEFAULT_LIMITER_THRESHOLD + 1e-6,
            "a step to 2.0 peaked at {worst}, above the {DEFAULT_LIMITER_THRESHOLD} ceiling"
        );
        assert!(
            (out[0].abs() - DEFAULT_LIMITER_THRESHOLD).abs() < 1e-6,
            "the very first sample must already be limited, got {}",
            out[0]
        );
    }

    /// A transient arriving mid-block is caught on its own sample, not after
    /// the envelope has had time to react.
    #[test]
    fn a_transient_is_caught_on_arrival() {
        let mut sink = AudioOutputSink::new(44100.0);
        let mut samples = vec![0.0f32; 512];
        samples[256] = 4.0;
        let out = feed(&mut sink, &samples);
        assert!(
            out[256].abs() <= DEFAULT_LIMITER_THRESHOLD + 1e-6,
            "a lone 4.0 spike came out at {}",
            out[256]
        );
    }

    /// Signal that already fits is passed through untouched — a ceiling must
    /// not become a compressor on quiet material.
    #[test]
    fn material_under_the_ceiling_is_untouched() {
        let mut sink = AudioOutputSink::new(44100.0);
        let input: Vec<f32> = (0..512).map(|i| 0.5 * (i as f32 * 0.05).sin()).collect();
        let out = feed(&mut sink, &input);
        for (got, want) in out.iter().zip(&input) {
            assert!((got - want).abs() < 1e-6, "expected {want}, got {got}");
        }
    }

    /// The ceiling still holds across a block boundary, where the gain state
    /// has to carry over.
    #[test]
    fn the_ceiling_holds_across_blocks() {
        let mut sink = AudioOutputSink::new(44100.0);
        feed(&mut sink, &[0.0; 512]);
        let out = feed(&mut sink, &[3.0; 512]);
        let worst = out.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(
            worst <= DEFAULT_LIMITER_THRESHOLD + 1e-6,
            "peaked at {worst}"
        );
    }

    /// Master volume is applied before the ceiling, so turning it up cannot
    /// push the output past the ceiling.
    #[test]
    fn master_volume_cannot_push_past_the_ceiling() {
        let mut sink = AudioOutputSink::new(44100.0);
        sink.set_parameter("master_volume", 8.0);
        let out = feed(&mut sink, &[0.5; 256]);
        let worst = out.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(
            worst <= DEFAULT_LIMITER_THRESHOLD + 1e-6,
            "peaked at {worst}"
        );
    }

    #[test]
    fn telemetry_reports_the_signal_before_and_after_limiting() {
        let mut sink = AudioOutputSink::new(44100.0);
        let out = feed(&mut sink, &[2.0, 0.25]);
        let telemetry = sink.telemetry().expect("audio output exposes telemetry");

        assert!((telemetry.pre_limiter_peak - 2.0).abs() < 1e-6);
        assert!((telemetry.post_limiter_peak - out[0].abs()).abs() < 1e-6);
        assert!(telemetry.max_gain_reduction_db > 6.0);
        assert!(telemetry.limited_samples >= CHANNELS);
    }
}
