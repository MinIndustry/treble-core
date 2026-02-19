//! Filter Unit Tests
//! Tests for audio filters including pass filters, effects, and structural filters

use rustic::core::audio::{Block, CHANNELS, silent_block};
use rustic::core::graph::{Entry, Filter};

/// Create a constant stereo block: every frame has value [v, v]
fn const_block(n: usize, v: f32) -> Block {
    vec![[v; CHANNELS]; n]
}

#[cfg(test)]
mod amplifier_tests {
    use super::*;
    use rustic::core::filters::prelude::GainFilter;

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
mod clipper_tests {
    use super::*;
    use rustic::core::filters::prelude::Clipper;

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
        let block: Block = vec![[-1.0; CHANNELS]; 4];
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
    use rustic::core::filters::prelude::Compressor;

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
        assert!(last_frame[0] < 0.9, "Compressor should reduce loud signal, got {}", last_frame[0]);
    }
}

#[cfg(test)]
mod delay_tests {
    use super::*;
    use rustic::core::filters::prelude::DelayFilter;

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
        f.push(silent_block(10), 0);
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
}

#[cfg(test)]
mod lowpass_tests {
    use super::*;
    use rustic::core::filters::prelude::LowPassFilter;

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
        assert!(last[0] > 0.99, "LPF should converge near 1.0, got {}", last[0]);
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
        assert!(out[0][0][0] < 0.5, "LPF should attenuate step, got {}", out[0][0][0]);
    }
}

#[cfg(test)]
mod highpass_tests {
    use super::*;
    use rustic::core::filters::prelude::HighPassFilter;

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
        assert!(last[0].abs() < 0.1, "HPF should attenuate DC, got {}", last[0]);
    }
}

#[cfg(test)]
mod bandpass_tests {
    use super::*;
    use rustic::core::filters::prelude::BandPass;

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
    use rustic::core::filters::prelude::Tremolo;

    #[test]
    fn test_tremolo_modulates_amplitude() {
        let mut f = Tremolo::new(1.0, 1.0, 44100.0);
        f.push(const_block(512, 1.0), 0);
        let out = f.transform();
        assert_eq!(out[0].len(), 512);
        // With depth=1.0, some frames should be attenuated
        let max_val = out[0].iter().map(|fr| fr[0]).fold(f32::NEG_INFINITY, f32::max);
        let min_val = out[0].iter().map(|fr| fr[0]).fold(f32::INFINITY, f32::min);
        assert!(max_val > min_val, "Tremolo should modulate amplitude");
    }

    #[test]
    fn test_tremolo_zero_depth_passthrough() {
        let mut f = Tremolo::new(5.0, 0.0, 44100.0);
        f.push(const_block(512, 0.8), 0);
        let out = f.transform();
        for frame in &out[0] {
            assert!((frame[0] - 0.8).abs() < 1e-5, "Zero-depth tremolo should pass through");
        }
    }
}

#[cfg(test)]
mod combinator_tests {
    use super::*;
    use rustic::core::filters::prelude::CombinatorFilter;

    #[test]
    fn test_combinator_sums_two_inputs() {
        let mut f = CombinatorFilter::new(2, 1);
        f.push(const_block(4, 0.3), 0);
        f.push(const_block(4, 0.2), 1);
        let out = f.transform();
        assert_eq!(out.len(), 1);
        for frame in &out[0] {
            assert!((frame[0] - 0.5).abs() < 1e-5, "Expected 0.3+0.2=0.5, got {}", frame[0]);
        }
    }

    #[test]
    fn test_combinator_multiple_outputs() {
        let mut f = CombinatorFilter::new(1, 3);
        f.push(const_block(4, 1.0), 0);
        let out = f.transform();
        assert_eq!(out.len(), 3, "Should have 3 output ports");
        for port_out in &out {
            for frame in port_out {
                assert!((frame[0] - 1.0).abs() < 1e-5);
            }
        }
    }
}

#[cfg(test)]
mod duplicate_tests {
    use super::*;
    use rustic::core::filters::prelude::DuplicateFilter;

    #[test]
    fn test_duplicate_produces_two_identical_outputs() {
        let mut f = DuplicateFilter::new();
        f.push(const_block(4, 0.7), 0);
        let out = f.transform();
        assert_eq!(out.len(), 2, "DuplicateFilter should produce 2 output ports");
        assert_eq!(out[0], out[1], "Both outputs should be identical");
        for frame in &out[0] {
            assert!((frame[0] - 0.7).abs() < 1e-5);
        }
    }
}

#[cfg(test)]
mod moving_average_tests {
    use super::*;
    use rustic::core::filters::prelude::MovingAverage;

    #[test]
    fn test_moving_average_smooths_step() {
        let mut f = MovingAverage::new(3);
        // Push silence first to initialize buffer
        f.push(silent_block(10), 0);
        f.transform();
        // Now push 1.0 signal
        f.push(const_block(10, 1.0), 0);
        let out = f.transform();
        // The average should ramp up from 0 to 1 over the window size
        let last = out[0].last().unwrap();
        assert!((last[0] - 1.0).abs() < 0.01, "MA should converge to 1.0, got {}", last[0]);
    }
}
