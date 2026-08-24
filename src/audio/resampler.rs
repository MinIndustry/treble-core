//! Render-side output resampling.

use std::collections::VecDeque;

/// Streaming stereo linear interpolator used between the engine and device rates.
pub(crate) struct LinearResampler {
    step: f64,
    position: f64,
    frames: VecDeque<[f32; 2]>,
}

impl LinearResampler {
    pub fn new(input_rate: u32, output_rate: u32) -> Self {
        Self {
            step: input_rate.max(1) as f64 / output_rate.max(1) as f64,
            position: 0.0,
            frames: VecDeque::new(),
        }
    }

    pub fn process(&mut self, input: &[f32], output: &mut Vec<f32>) {
        output.clear();
        if self.step == 1.0 {
            output.extend_from_slice(input);
            return;
        }
        // as_chunks yields [f32; 2] directly, which is already a Frame; the
        // trailing remainder (an odd sample) is discarded as before.
        self.frames.extend(input.as_chunks::<2>().0.iter().copied());

        while self.position + 1.0 < self.frames.len() as f64 {
            let index = self.position.floor() as usize;
            let fraction = (self.position - index as f64) as f32;
            let a = self.frames[index];
            let b = self.frames[index + 1];
            output.push(a[0] + (b[0] - a[0]) * fraction);
            output.push(a[1] + (b[1] - a[1]) * fraction);
            self.position += self.step;
        }

        let consumed = self.position.floor() as usize;
        for _ in 0..consumed.min(self.frames.len().saturating_sub(1)) {
            self.frames.pop_front();
        }
        self.position -= consumed as f64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_stereo_channels_when_upsampling() {
        let mut resampler = LinearResampler::new(24_000, 48_000);
        let mut output = Vec::new();
        resampler.process(&[0.0, 1.0, 1.0, 0.0, 0.0, -1.0], &mut output);
        assert_eq!(&output[..4], &[0.0, 1.0, 0.5, 0.5]);
        assert!(output.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn keeps_fractional_position_across_blocks() {
        let mut resampler = LinearResampler::new(44_100, 48_000);
        let mut first = Vec::new();
        let mut second = Vec::new();
        resampler.process(&[0.0, 0.0, 1.0, 1.0], &mut first);
        resampler.process(&[2.0, 2.0, 3.0, 3.0], &mut second);
        assert!(!first.is_empty());
        assert!(!second.is_empty());
        assert!(second[0] >= first[first.len() - 2]);
    }
}
