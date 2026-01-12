//! ADSR Envelope Tests
//! Tests for ADSR envelope generation and behavior

#[cfg(test)]
mod tests {
    use rustic::core::envelope::{Envelope, prelude::{ADSREnvelope, ADSREnvelopeBuilder, LinearSegment}};

    // TODO: Add tests for ADSR envelope functionality
    #[test]
    fn test_adsr_phase_transition() {
        let adsr_envelope: ADSREnvelope = ADSREnvelopeBuilder::new()
            .attack(Box::new(LinearSegment::new(0.0, 1.0, 1.0)))
            .decay(Box::new(LinearSegment::new(1.0, 0.8, 0.2)))
            .release(Box::new(LinearSegment::new(0.8, 0.0, 1.0)))
            .build();

        let attack_mapping = adsr_envelope.attack.map_time(0.0, 0.5);

        assert!(attack_mapping < 0.51 && attack_mapping > 0.49, "attack mapping should be around 0.5");
        assert_eq!(adsr_envelope.attack.get_duration(), 1.0, "Attack segment should last 1.0 seconds");

        // assert!(false, "ADSR Envelope: {}", adsr_envelope);

        let decay_mapping = adsr_envelope.decay.map_time(adsr_envelope.attack.get_duration(), 1.1);

        assert!(decay_mapping < 0.51 && decay_mapping > 0.49, "Decay mapping should be around 0.5");
        assert_eq!(adsr_envelope.decay.get_duration(), 0.2, "Decay segment should last 0.2 seconds");

        let release_mapping = adsr_envelope.release.map_time(1.0, 1.5);

        assert!(release_mapping < 0.51 && release_mapping > 0.49, "Release mapping should be around 0.5");

        let envelope = Box::new(adsr_envelope);

        let mid_attack = envelope.at(0.5, 0.0);
        let end_attack = envelope.at(1.0, 0.0);

        let mid_decay = envelope.at(1.1, 0.0);
        let end_decay = envelope.at(1.2, 0.0);

        assert!(mid_attack > 0.49 && mid_attack < 0.51, "Center value of the attack should be around 0.5 (reality {})", mid_attack);
        assert!(end_attack > 0.99 && end_attack < 1.01, "Attack should end around 1.0 (reality: {})", end_attack);
        assert!(mid_decay > 0.89 && mid_decay < 0.91, "Middle of the decay segment should be 0.9 (reality: {})", mid_decay);
        assert!(end_decay > 0.79 && end_decay < 0.81, "End of the decay segment should be 0.8 (reality: {})", end_decay);
    }

    // - Test ADSR phase transitions (Attack -> Decay -> Sustain -> Release)
    // - Test envelope timing and duration calculations
    // - Test sustain level behavior
    // - Test release after various hold durations
}
