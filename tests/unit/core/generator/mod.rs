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
    // TODO: Add tests for different waveforms
    // - Test sine wave generation
    // - Test square wave generation
    // - Test triangle wave generation
    // - Test sawtooth wave generation
    // - Test noise generation
    // - Test blank/silence generation
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
