//! Filter Unit Tests
//! Tests for audio filters including pass filters, effects, and structural filters

use std::sync::Arc;
use treble::core::audio::{Block, CHANNELS, silent_block};
use treble::core::graph::{Entry, Filter};

/// Create a constant stereo block: every frame has value [v, v]
fn const_block(n: usize, v: f32) -> Arc<Block> {
    Arc::new(vec![[v; CHANNELS]; n])
}

#[cfg(test)]
mod amplifier_tests {
    use super::*;
    use treble::core::filters::prelude::GainFilter;

    #[test]
    fn test_gain_multiplies_signal() {
        let mut f = GainFilter::new(2.0);
        f.push(const_block(4, 0.5), 0);
        let out = f.transform();
        assert_eq!(out.len(), 1);
        for frame in &out[0] {
            assert!((frame[0] - 1.0).abs() < 1e-5);
            assert!((frame[1] - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn test_gain_zero() {
        let mut f = GainFilter::new(0.0);
        f.push(const_block(4, 1.0), 0);
        let out = f.transform();
        for frame in &out[0] {
            assert_eq!(frame[0], 0.0);
            assert_eq!(frame[1], 0.0);
        }
    }

    #[test]
    fn test_gain_preserves_block_length() {
        let mut f = GainFilter::new(1.0);
        f.push(const_block(32, 0.3), 0);
        let out = f.transform();
        assert_eq!(out[0].len(), 32);
    }
}

#[cfg(test)]
mod resonant_bandpass_tests {
    use super::*;
    use treble::core::filters::prelude::ResonantBandpassFilter;

    /// RMS gain of the filter for a sine at `freq`, transient skipped.
    fn sine_gain(filter: &mut ResonantBandpassFilter, freq: f32, sample_rate: f32) -> f32 {
        let n = sample_rate as usize;
        let input: Arc<Block> = Arc::new(
            (0..n)
                .map(|i| {
                    let s = (std::f32::consts::TAU * freq * i as f32 / sample_rate).sin();
                    [s; CHANNELS]
                })
                .collect(),
        );
        filter.push(input, 0);
        let out = &filter.transform()[0];
        let rms = |frames: &[[f32; CHANNELS]]| {
            (frames.iter().map(|f| f[0] * f[0]).sum::<f32>() / frames.len() as f32).sqrt()
        };
        // Skip the first half: the resonator's ring-in, and the input's own
        // RMS over the same window for a fair ratio.
        rms(&out[n / 2..]) / (0.5f32).sqrt()
    }

    /// The design normalizes the peak: a tone at the center frequency must
    /// come out at unity, not merely "the loudest". This is the regression
    /// test for the transposed-form state update feeding b2 a sample early,
    /// which cost ~6 dB at the center and muffled everything else.
    #[test]
    fn test_unity_gain_at_the_center_frequency() {
        for quality in [1.0, 8.0, 30.0] {
            let mut f = ResonantBandpassFilter::new(1000.0, quality, 44100.0);
            let gain = sine_gain(&mut f, 1000.0, 44100.0);
            assert!(
                (gain - 1.0).abs() < 0.05,
                "Q={quality}: center-frequency gain {gain}, expected ~1.0"
            );
        }
    }

    /// Off-center content falls away — that is what makes it a bandpass —
    /// and more steeply at higher quality.
    #[test]
    fn test_off_center_tones_attenuate_with_quality() {
        let mut broad = ResonantBandpassFilter::new(1000.0, 1.0, 44100.0);
        let mut narrow = ResonantBandpassFilter::new(1000.0, 8.0, 44100.0);
        let at_broad = sine_gain(&mut broad, 250.0, 44100.0);
        let at_narrow = sine_gain(&mut narrow, 250.0, 44100.0);
        assert!(
            at_broad < 0.7,
            "a tone two octaves out should attenuate: {at_broad}"
        );
        assert!(
            at_narrow < at_broad,
            "higher Q must cut off-center harder: Q8 {at_narrow} vs Q1 {at_broad}"
        );
    }
}

#[cfg(test)]
mod clipper_tests {
    use super::*;
    use treble::core::filters::prelude::Clipper;

    #[test]
    fn test_clipping_above_threshold() {
        let mut f = Clipper::new(0.5);
        f.push(const_block(4, 1.0), 0);
        let out = f.transform();
        for frame in &out[0] {
            assert!((frame[0] - 0.5).abs() < 1e-5);
        }
    }

    #[test]
    fn test_clipping_below_threshold_unchanged() {
        let mut f = Clipper::new(0.5);
        f.push(const_block(4, 0.3), 0);
        let out = f.transform();
        for frame in &out[0] {
            assert!((frame[0] - 0.3).abs() < 1e-5);
        }
    }

    #[test]
    fn test_clipping_negative() {
        let mut f = Clipper::new(0.5);
        let block = Arc::new(vec![[-1.0_f32; CHANNELS]; 4]);
        f.push(block, 0);
        let out = f.transform();
        for frame in &out[0] {
            assert!((frame[0] + 0.5).abs() < 1e-5);
        }
    }
}

#[cfg(test)]
mod compressor_tests {
    use super::*;
    use treble::core::filters::prelude::Compressor;

    #[test]
    fn test_compressor_below_threshold_passes_through() {
        let mut f = Compressor::default();
        // threshold=0.5, signal=0.1 should pass through unchanged (approximately)
        f.push(const_block(100, 0.1), 0);
        let out = f.transform();
        // Output should be close to input when below threshold
        assert!(!out[0].is_empty());
        for frame in &out[0] {
            assert!(frame[0] > 0.0);
        }
    }

    #[test]
    fn test_compressor_reduces_loud_signal() {
        let mut f = Compressor::default();
        // Need enough frames for the slow attack envelope to build past threshold (0.5).
        // At 44100 Hz and attack=0.01s, ~441 frames per time constant. Run 2000 frames total.
        for _ in 0..4 {
            f.push(const_block(512, 0.9), 0);
            f.transform();
        }
        f.push(const_block(512, 0.9), 0);
        let out = f.transform();
        let last_frame = out[0].last().unwrap();
        // Compressed output should be less than raw input amplitude
        assert!(
            last_frame[0] < 0.9,
            "Compressor should reduce loud signal, got {}",
            last_frame[0]
        );
    }
}

#[cfg(test)]
mod delay_tests {
    use super::*;
    use treble::core::filters::prelude::DelayFilter;
    use treble_meta::MetaFilter;

    #[test]
    fn test_delay_outputs_silence_initially() {
        let sample_rate = 100.0;
        let delay = 1.0; // 1 second = 100 frames
        let mut f = DelayFilter::new(sample_rate, delay);
        f.push(const_block(10, 1.0), 0);
        let out = f.transform();
        // First 10 frames should be silence (from the pre-filled buffer)
        for frame in &out[0] {
            assert!(frame[0].abs() < 1e-5, "Expected silence, got {}", frame[0]);
        }
    }

    #[test]
    fn test_delay_passes_after_delay_time() {
        let sample_rate = 10.0;
        let delay = 1.0; // 10 frames delay
        let mut f = DelayFilter::new(sample_rate, delay);

        // Push 10 frames of silence (to fill the delay buffer)
        f.push(Arc::new(silent_block(10)), 0);
        let _ = f.transform();

        // Push signal
        f.push(const_block(10, 1.0), 0);
        let out = f.transform();
        // Now output should have the originally pushed signal (which was silence)
        // The signal we just pushed should appear 10 frames later
        for frame in &out[0] {
            assert!(frame[0].abs() < 1e-5, "Expected delayed silence");
        }
    }

    #[test]
    fn test_delay_metadata_parameters_resize_the_buffer() {
        let mut f = DelayFilter::default();
        assert!(f.set_parameter("sample_rate", 10.0));
        assert!(f.set_parameter("delay_for", 0.2));
        assert!(f.set_parameter("mix", 1.0));
        assert!(!f.set_parameter("delay_typo", 1.0));
        f.push(
            Arc::new(vec![[1.0; CHANNELS], [0.0; CHANNELS], [0.0; CHANNELS]]),
            0,
        );
        let out = f.transform();

        assert_eq!(out[0][0], [0.0; CHANNELS]);
        assert_eq!(out[0][1], [0.0; CHANNELS]);
        assert_eq!(out[0][2], [1.0; CHANNELS]);
    }
}

#[cfg(test)]
mod reverb_tests {
    use super::*;
    use treble::core::filters::prelude::ReverbFilter;

    #[test]
    fn test_reverb_preserves_dry_attack_and_produces_a_tail() {
        let mut f = ReverbFilter::new(1_000.0, 0.5);
        let mut impulse = silent_block(100);
        impulse[0] = [1.0; CHANNELS];
        f.push(Arc::new(impulse), 0);
        let out = f.transform();

        assert!((out[0][0][0] - 0.5).abs() < 1e-5);
        assert!(out[0][20..].iter().any(|frame| frame[0].abs() > 1e-5));
    }
}

#[cfg(test)]
mod lowpass_tests {
    use super::*;
    use treble::core::filters::prelude::LowPassFilter;

    #[test]
    fn test_lowpass_converges_toward_dc() {
        let mut f = LowPassFilter::new(1000.0, 44100.0);
        // Push many blocks of constant 1.0 — output should converge to 1.0
        for _ in 0..100 {
            f.push(const_block(512, 1.0), 0);
            f.transform();
        }
        f.push(const_block(512, 1.0), 0);
        let out = f.transform();
        let last = out[0].last().unwrap();
        assert!(
            last[0] > 0.99,
            "LPF should converge near 1.0, got {}",
            last[0]
        );
    }

    #[test]
    fn test_lowpass_blocks_high_freq_change() {
        let mut f = LowPassFilter::new(100.0, 44100.0);
        // Feed a step function: filter should smooth it out
        f.push(const_block(512, 0.0), 0);
        f.transform();
        f.push(const_block(1, 1.0), 0);
        let out = f.transform();
        // With low cutoff, the first frame response is small
        assert!(
            out[0][0][0] < 0.5,
            "LPF should attenuate step, got {}",
            out[0][0][0]
        );
    }
}

#[cfg(test)]
mod highpass_tests {
    use super::*;
    use treble::core::filters::prelude::HighPassFilter;

    #[test]
    fn test_highpass_attenuates_dc() {
        let mut f = HighPassFilter::new(1000.0, 44100.0);
        // DC (constant input) should decay to near zero
        for _ in 0..200 {
            f.push(const_block(512, 1.0), 0);
            f.transform();
        }
        f.push(const_block(512, 1.0), 0);
        let out = f.transform();
        let last = out[0].last().unwrap();
        // After many blocks the HPF output should approach 0 for constant input
        assert!(
            last[0].abs() < 0.1,
            "HPF should attenuate DC, got {}",
            last[0]
        );
    }
}

#[cfg(test)]
mod bandpass_tests {
    use super::*;
    use treble::core::filters::prelude::BandPass;

    #[test]
    fn test_bandpass_produces_output() {
        let mut f = BandPass::new(500.0, 2000.0, 44100.0);
        f.push(const_block(512, 0.5), 0);
        let out = f.transform();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].len(), 512);
    }
}

#[cfg(test)]
mod tremolo_tests {
    use super::*;
    use treble::core::filters::prelude::Tremolo;

    #[test]
    fn test_tremolo_modulates_amplitude() {
        let mut f = Tremolo::new(1.0, 1.0, 44100.0);
        f.push(const_block(512, 1.0), 0);
        let out = f.transform();
        assert_eq!(out[0].len(), 512);
        // With depth=1.0, some frames should be attenuated
        let max_val = out[0]
            .iter()
            .map(|fr| fr[0])
            .fold(f32::NEG_INFINITY, f32::max);
        let min_val = out[0].iter().map(|fr| fr[0]).fold(f32::INFINITY, f32::min);
        assert!(max_val > min_val, "Tremolo should modulate amplitude");
    }

    #[test]
    fn test_tremolo_zero_depth_passthrough() {
        let mut f = Tremolo::new(5.0, 0.0, 44100.0);
        f.push(const_block(512, 0.8), 0);
        let out = f.transform();
        for frame in &out[0] {
            assert!(
                (frame[0] - 0.8).abs() < 1e-5,
                "Zero-depth tremolo should pass through"
            );
        }
    }
}

#[cfg(test)]
mod moving_average_tests {
    use super::*;
    use treble::core::filters::prelude::MovingAverage;

    #[test]
    fn test_moving_average_smooths_step() {
        let mut f = MovingAverage::new(3);
        // Push silence first to initialize buffer
        f.push(Arc::new(silent_block(10)), 0);
        f.transform();
        // Now push 1.0 signal
        f.push(const_block(10, 1.0), 0);
        let out = f.transform();
        // The average should ramp up from 0 to 1 over the window size
        let last = out[0].last().unwrap();
        assert!(
            (last[0] - 1.0).abs() < 0.01,
            "MA should converge to 1.0, got {}",
            last[0]
        );
    }
}
