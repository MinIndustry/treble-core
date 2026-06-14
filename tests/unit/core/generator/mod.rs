//! Generator Unit Tests
//! Tests for tone generators and waveform generation

#[cfg(test)]
mod tone_generator_tests {
    // TODO: Add tests for ToneGenerator
    // - Test frequency accuracy
    // - Test phase continuity
    // - Test start/stop behavior

    use treble::core::generator::prelude::FrequencyRelation;
    const BASE_TONE: f32 = 440.0;

    #[test]
    pub fn test_frequency_relation_identity() {
        assert_eq!(
            FrequencyRelation::Identity.compute(BASE_TONE),
            BASE_TONE,
            "Indentity frequency relation should return the base frequency"
        );
    }

    #[test]
    pub fn test_frequency_relation_constant() {
        assert_eq!(
            FrequencyRelation::Constant(200.0).compute(BASE_TONE),
            200.0,
            "Constant frequency relation should return the constant and ignore the base frequency"
        );
    }

    #[test]
    pub fn test_frequency_relation_harmonic() {
        for factor in [2, 4, 10] {
            assert_eq!(
                FrequencyRelation::Harmonic(factor).compute(BASE_TONE),
                BASE_TONE * (factor as f32),
                "The {factor}(tf) harmonic should be {factor}*base_freq"
            );
        }
    }

    #[test]
    pub fn test_frequency_relation_ratio() {
        for ratio in [0.2, 0.4, 0.8, 1.2] {
            assert_eq!(
                FrequencyRelation::Ratio(ratio).compute(BASE_TONE),
                BASE_TONE * ratio,
                "Ratio {ratio} frequency relation should be {ratio}*base_freq"
            );
        }
    }

    #[test]
    pub fn test_frequency_relation_offset() {
        for offset in [1.0, 20.0, 100.0, -2.0, -4.0] {
            assert_eq!(
                FrequencyRelation::Offset(offset).compute(BASE_TONE),
                BASE_TONE + offset,
                "Offset {offset} frequency relation should be base_freq+{offset}"
            );
        }
    }

    #[test]
    pub fn test_frequency_relation_semitones() {
        assert_eq!(
            FrequencyRelation::Semitones(12).compute(BASE_TONE),
            FrequencyRelation::Harmonic(2).compute(BASE_TONE),
            "12 semitones relation should equal an octave or the second harmonic"
        );
        assert_eq!(
            FrequencyRelation::Semitones(-12).compute(BASE_TONE),
            FrequencyRelation::Ratio(0.5).compute(BASE_TONE),
            "-12 semitones relation should equal a halfing"
        );

        let semitone_result = FrequencyRelation::Semitones(1).compute(BASE_TONE);
        let approx_expected = BASE_TONE * 1.059463;
        assert!(
            (semitone_result - approx_expected).abs() < 0.01,
            "A single semitone difference should be close to a ratio of 1.059463 ({semitone_result} != {approx_expected})"
        );
    }
}

#[cfg(test)]
mod waveform_tests {
    use treble::core::generator::prelude::{Waveform, builder::ToneGeneratorBuilder};

    const SAMPLE_RATE: f32 = 44100.0;
    const TIME_STEP: f32 = 1.0 / SAMPLE_RATE;
    // 2 kHz stresses aliasing; high enough that naive waveforms have noticeable
    // discontinuities but low enough that polyBLEP/BLAMP stays in its valid range.
    const FREQ: f32 = 2000.0;
    const RENDER_SAMPLES: usize = 4096;

    fn render(waveform: Waveform) -> Vec<f32> {
        let mut tone = ToneGeneratorBuilder::new()
            .waveform(waveform)
            .frequency(FREQ)
            .build();
        tone.start();
        (0..RENDER_SAMPLES).map(|_| tone.tick(TIME_STEP)).collect()
    }

    fn max_abs(samples: &[f32]) -> f32 {
        samples.iter().copied().fold(0.0_f32, |m, v| m.max(v.abs()))
    }

    fn max_consecutive_diff(samples: &[f32]) -> f32 {
        samples
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0.0_f32, f32::max)
    }

    #[test]
    fn band_limited_saw_amplitude_bounded() {
        let samples = render(Waveform::Sawtooth);
        // The polyBLEP correction can overshoot slightly; allow up to 1.05
        assert!(
            max_abs(&samples) <= 1.05,
            "band-limited saw exceeded amplitude bound: {:.4}",
            max_abs(&samples)
        );
    }

    #[test]
    fn band_limited_square_amplitude_bounded() {
        let samples = render(Waveform::Square);
        assert!(
            max_abs(&samples) <= 1.05,
            "band-limited square exceeded amplitude bound: {:.4}",
            max_abs(&samples)
        );
    }

    #[test]
    fn band_limited_triangle_amplitude_bounded() {
        let samples = render(Waveform::Triangle);
        assert!(
            max_abs(&samples) <= 1.05,
            "band-limited triangle exceeded amplitude bound: {:.4}",
            max_abs(&samples)
        );
    }

    #[test]
    fn band_limited_saw_smaller_discontinuity_than_raw() {
        let blep = render(Waveform::Sawtooth);
        let raw = render(Waveform::SawRaw);
        let blep_jump = max_consecutive_diff(&blep);
        let raw_jump = max_consecutive_diff(&raw);
        assert!(
            blep_jump < raw_jump,
            "band-limited saw (max diff {blep_jump:.4}) should have smaller jumps than raw ({raw_jump:.4})"
        );
    }

    #[test]
    fn band_limited_square_smaller_discontinuity_than_raw() {
        let blep = render(Waveform::Square);
        let raw = render(Waveform::SquareRaw);
        let blep_jump = max_consecutive_diff(&blep);
        let raw_jump = max_consecutive_diff(&raw);
        assert!(
            blep_jump < raw_jump,
            "band-limited square (max diff {blep_jump:.4}) should have smaller jumps than raw ({raw_jump:.4})"
        );
    }
}

#[cfg(test)]
mod composite_generator_tests {
    use treble::core::generator::prelude::builder::MultiToneGeneratorBuilder;

    #[test]
    pub fn test_tick_block_consistency() {
        const NUM_SAMPLES: usize = 10;
        const PERIOD: f32 = 1.0 / 44100.0;

        let mut generator_1 = MultiToneGeneratorBuilder::new().build();
        let mut generator_2 = MultiToneGeneratorBuilder::new().build();

        generator_1.start();
        generator_2.start();

        let samples = generator_1.tick_block(NUM_SAMPLES, PERIOD);
        for sample in samples.iter().take(NUM_SAMPLES) {
            assert_eq!(*sample, generator_2.tick(PERIOD));
        }
    }
}

#[cfg(test)]
mod builder_tests {
    // TODO: Add tests for ToneGeneratorBuilder
    // - Test builder pattern construction
    // - Test default values
    // - Test parameter validation
}
