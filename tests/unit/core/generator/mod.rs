//! Generator Unit Tests
//! Tests for tone generators and waveform generation

#[cfg(test)]
mod tone_generator_tests {
    // TODO: Add tests for ToneGenerator
    // - Test frequency accuracy
    // - Test phase continuity
    // - Test start/stop behavior
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
    use rustic::core::generator::prelude::builder::MultiToneGeneratorBuilder;

    #[test]
    pub fn test_tick_block_consistency() {
        const NUM_SAMPLES: usize = 10;
        const PERIOD: f32 = 1.0 / 44100.0;

        let mut generator_1 = MultiToneGeneratorBuilder::new().build();
        let mut generator_2 = MultiToneGeneratorBuilder::new().build();

        generator_1.start();
        generator_2.start();

        let samples = generator_1.tick_block(NUM_SAMPLES, PERIOD);
        for i in 0..NUM_SAMPLES {
            assert_eq!(samples[i], generator_2.tick(PERIOD));
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
