//! Render-side output resampling.
//!
//! The engine renders at its own rate and the device asks for whatever it asks
//! for — 48 kHz on most Linux stacks, where the engine prefers 44.1 kHz — so
//! everything between the two passes through here.
//!
//! This used to be linear interpolation, which is a two-tap filter with no
//! stopband: on the way down it folded everything above the new Nyquist back
//! into the audible band, and on the way up it left images of the whole
//! spectrum sitting just above it. Percussion is built from noise and carries
//! energy right up to Nyquist, so that fold-back was audible as a metallic
//! edge that appeared only when the device rate differed from the engine's —
//! which is exactly the case that made it look like a platform bug.
//!
//! What replaces it is a windowed-sinc polyphase interpolator: a band-limited
//! reconstruction of the input, sampled at the output's instants. The kernel is
//! computed once into a phase table, so the per-sample cost is a fixed
//! [`TAPS`]-tap dot product with no transcendental maths on the audio path.

use std::collections::VecDeque;

/// Kernel width in input samples. 32 taps puts the first sidelobe far enough
/// down that the fold-back is inaudible, at a cost the render thread can pay:
/// one 32-tap dot product per channel per output sample.
const TAPS: usize = 32;
/// Half-width, in whole input samples either side of the read position.
const HALF: usize = TAPS / 2;
/// Sub-sample positions the kernel is precomputed at. At 512 the phase
/// quantisation error is far below the 32-tap kernel's own stopband, so the
/// nearest phase can be used directly rather than interpolated between two.
const PHASES: usize = 512;
/// How much of the available band the kernel passes before it starts rolling
/// off. Below 1.0 so the transition band lands under Nyquist rather than
/// straddling it.
const ROLLOFF: f64 = 0.90;

/// Streaming stereo band-limited resampler between the engine and device rates.
pub(crate) struct OutputResampler {
    /// Input frames consumed per output frame.
    step: f64,
    /// Fractional read position within `frames`.
    position: f64,
    frames: VecDeque<[f32; 2]>,
    /// `PHASES × TAPS` windowed-sinc kernel, row per sub-sample position.
    kernel: Option<Vec<f32>>,
}

/// `sin(pi x) / (pi x)`, defined at zero.
fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-9 {
        1.0
    } else {
        let pix = std::f64::consts::PI * x;
        pix.sin() / pix
    }
}

/// Blackman window over `0..=1`. Chosen over a plain Hann for its lower
/// sidelobes, which is what sets how much of the stopband actually folds back.
fn blackman(t: f64) -> f64 {
    let two_pi_t = 2.0 * std::f64::consts::PI * t;
    0.42 - 0.5 * two_pi_t.cos() + 0.08 * (2.0 * two_pi_t).cos()
}

/// Build the phase table for a given conversion ratio.
///
/// `cutoff` is in cycles per input sample. Going up, the whole input band is
/// already valid and the kernel only has to reconstruct it; coming down, the
/// kernel doubles as the anti-aliasing filter and has to cut at the *output's*
/// Nyquist instead.
fn build_kernel(step: f64) -> Vec<f32> {
    let cutoff = 0.5 * ROLLOFF * if step > 1.0 { 1.0 / step } else { 1.0 };
    let mut kernel = vec![0.0f32; PHASES * TAPS];
    for phase in 0..PHASES {
        let frac = phase as f64 / PHASES as f64;
        let row = &mut kernel[phase * TAPS..(phase + 1) * TAPS];
        let mut sum = 0.0f64;
        for (tap, value) in row.iter_mut().enumerate() {
            // Offset of this input sample from the read position.
            let t = (tap as f64 - HALF as f64 + 1.0) - frac;
            let window = blackman((t + HALF as f64) / TAPS as f64);
            let h = 2.0 * cutoff * sinc(2.0 * cutoff * t) * window;
            *value = h as f32;
            sum += h;
        }
        // Normalise each phase to unity DC gain, so a constant input comes out
        // constant and the conversion cannot change the level.
        if sum.abs() > 1e-12 {
            for value in row.iter_mut() {
                *value = (*value as f64 / sum) as f32;
            }
        }
    }
    kernel
}

impl OutputResampler {
    pub fn new(input_rate: u32, output_rate: u32) -> Self {
        let step = input_rate.max(1) as f64 / output_rate.max(1) as f64;
        Self {
            step,
            // Start far enough in that the kernel has left-hand context; the
            // priming silence below supplies it.
            position: 0.0,
            frames: VecDeque::new(),
            // Matched rates short-circuit, so they need no table at all.
            kernel: (step != 1.0).then(|| build_kernel(step)),
        }
    }

    pub fn process(&mut self, input: &[f32], output: &mut Vec<f32>) {
        output.clear();
        let Some(kernel) = &self.kernel else {
            output.extend_from_slice(input);
            return;
        };

        if self.frames.is_empty() {
            // Prime the kernel's left half with silence so the first output
            // sample is filtered like every other one rather than being a
            // partial sum, which would land as a click at stream start.
            for _ in 0..HALF {
                self.frames.push_back([0.0, 0.0]);
            }
            self.position = HALF as f64;
        }

        // as_chunks yields [f32; 2] directly, which is already a Frame; the
        // trailing remainder (an odd sample) is discarded as before.
        self.frames.extend(input.as_chunks::<2>().0.iter().copied());

        // An output sample needs HALF frames of lookahead past its position.
        while self.position + HALF as f64 + 1.0 < self.frames.len() as f64 {
            let base = self.position.floor() as usize;
            let frac = self.position - base as f64;
            let phase = ((frac * PHASES as f64) as usize).min(PHASES - 1);
            let row = &kernel[phase * TAPS..(phase + 1) * TAPS];

            let (mut left, mut right) = (0.0f32, 0.0f32);
            for (tap, weight) in row.iter().enumerate() {
                // Mirrors the offset used when the kernel was built.
                let index = base + tap + 1 - HALF;
                let frame = self.frames[index];
                left += frame[0] * weight;
                right += frame[1] * weight;
            }
            output.push(left);
            output.push(right);
            self.position += self.step;
        }

        // Retire frames the kernel can no longer reach, keeping its left half.
        let keep_from = (self.position.floor() as usize).saturating_sub(HALF);
        for _ in 0..keep_from.min(self.frames.len()) {
            self.frames.pop_front();
        }
        self.position -= keep_from as f64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Peak amplitude of a sine at `hz` in `samples`, by direct correlation.
    fn amplitude_at(samples: &[f32], hz: f64, rate: f64) -> f64 {
        let (mut re, mut im) = (0.0f64, 0.0f64);
        for (i, s) in samples.iter().enumerate() {
            let angle = 2.0 * std::f64::consts::PI * hz * i as f64 / rate;
            re += *s as f64 * angle.cos();
            im += *s as f64 * angle.sin();
        }
        2.0 * (re * re + im * im).sqrt() / samples.len() as f64
    }

    fn sine(hz: f64, rate: f64, frames: usize) -> Vec<f32> {
        (0..frames)
            .flat_map(|i| {
                let v = (2.0 * std::f64::consts::PI * hz * i as f64 / rate).sin() as f32;
                [v, v]
            })
            .collect()
    }

    #[test]
    fn matched_rates_pass_through_untouched() {
        let mut resampler = OutputResampler::new(48_000, 48_000);
        let mut output = Vec::new();
        let input = [0.0, 1.0, 0.5, -0.5, 0.25, 0.0];
        resampler.process(&input, &mut output);
        assert_eq!(output, input);
    }

    #[test]
    fn preserves_stereo_channels() {
        let mut resampler = OutputResampler::new(24_000, 48_000);
        let mut output = Vec::new();
        // Left and right carry different constants; they must not mix.
        let input: Vec<f32> = (0..4096).flat_map(|_| [1.0f32, -1.0f32]).collect();
        resampler.process(&input, &mut output);
        assert!(!output.is_empty());
        let settled = &output[512..];
        for pair in settled.chunks(2) {
            assert!((pair[0] - 1.0).abs() < 0.02, "left drifted to {}", pair[0]);
            assert!((pair[1] + 1.0).abs() < 0.02, "right drifted to {}", pair[1]);
        }
    }

    #[test]
    fn keeps_fractional_position_across_blocks() {
        let mut resampler = OutputResampler::new(44_100, 48_000);
        let input = sine(1_000.0, 44_100.0, 2048);
        let mut first = Vec::new();
        let mut second = Vec::new();
        resampler.process(&input[..2048], &mut first);
        resampler.process(&input[2048..], &mut second);
        assert!(!first.is_empty() && !second.is_empty());
        // Continuity: no step at the seam bigger than the waveform's own slope.
        let seam = (second[0] - first[first.len() - 2]).abs();
        assert!(seam < 0.2, "block seam jumped by {seam}");
    }

    /// A tone well inside the band survives the conversion at its own level.
    #[test]
    fn a_passband_tone_survives_upsampling() {
        let mut resampler = OutputResampler::new(44_100, 48_000);
        let mut output = Vec::new();
        resampler.process(&sine(1_000.0, 44_100.0, 16_384), &mut output);
        let left: Vec<f32> = output.chunks(2).map(|f| f[0]).collect();
        let settled = &left[TAPS..left.len() - TAPS];
        let amplitude = amplitude_at(settled, 1_000.0, 48_000.0);
        assert!(
            (amplitude - 1.0).abs() < 0.05,
            "1 kHz came through at {amplitude}, expected ~1.0"
        );
    }

    /// The point of the exercise: content above the output's Nyquist must be
    /// filtered out rather than folded back down into the audible band.
    ///
    /// Linear interpolation had no stopband, so an 18 kHz tone downsampled to
    /// 32 kHz reappeared at 14 kHz nearly intact. Percussion is noise and
    /// carries energy all the way up, so this was broadband, not one tone.
    #[test]
    fn content_above_the_new_nyquist_does_not_fold_back() {
        let mut resampler = OutputResampler::new(48_000, 32_000);
        let mut output = Vec::new();
        resampler.process(&sine(18_000.0, 48_000.0, 32_768), &mut output);
        let left: Vec<f32> = output.chunks(2).map(|f| f[0]).collect();
        let settled = &left[TAPS..left.len() - TAPS];
        // 18 kHz is above the 16 kHz output Nyquist; it would alias to 14 kHz.
        let alias = amplitude_at(settled, 14_000.0, 32_000.0);
        assert!(
            alias < 0.01,
            "an 18 kHz tone folded back to 14 kHz at amplitude {alias}"
        );
    }

    /// A constant input stays constant: the kernel has unity DC gain, so a
    /// rate change cannot alter the level of the material passing through it.
    #[test]
    fn dc_gain_is_unity() {
        let mut resampler = OutputResampler::new(44_100, 48_000);
        let mut output = Vec::new();
        let input: Vec<f32> = std::iter::repeat_n(0.5f32, 8192).collect();
        resampler.process(&input, &mut output);
        let settled = &output[TAPS * 2..];
        for sample in settled {
            assert!(
                (sample - 0.5).abs() < 0.001,
                "constant 0.5 came out as {sample}"
            );
        }
    }

    #[test]
    fn output_is_finite_and_roughly_the_right_length() {
        for (input_rate, output_rate) in [(44_100, 48_000), (48_000, 44_100), (96_000, 48_000)] {
            let mut resampler = OutputResampler::new(input_rate, output_rate);
            let mut output = Vec::new();
            let frames = 16_384;
            resampler.process(&sine(440.0, input_rate as f64, frames), &mut output);
            assert!(
                output.iter().all(|s| s.is_finite()),
                "{input_rate}->{output_rate}"
            );
            let produced = output.len() / 2;
            let expected = frames as f64 * output_rate as f64 / input_rate as f64;
            assert!(
                (produced as f64 - expected).abs() < TAPS as f64 * 2.0,
                "{input_rate}->{output_rate}: produced {produced}, expected about {expected}"
            );
        }
    }
}
