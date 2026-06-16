use treble::core::generator::prelude::{FrequencyRelation, MixMode, Waveform};
use treble::core::graph::MonophonicAllocationStrategy;
use treble::instruments::prelude::{EnvelopeSpec, InstrumentSpec, ToneSpec, VoiceSpec};
use treble::instruments::spec::compile_spec;

#[test]
pub fn test_kick_compilation() {
    let kick_spec: InstrumentSpec = InstrumentSpec {
        name: String::from("Kick"),
        voice: VoiceSpec::Mono {
            track_pitch: false,
            allocation: MonophonicAllocationStrategy::Replace,
        },
        tones: vec![
            ToneSpec {
                waveform: Waveform::WhiteNoise,
                frequency_relation: FrequencyRelation::Constant(1.0),
                amplitude_envelope: Some(EnvelopeSpec::Adsr {
                    attack: 0.01,
                    decay: 0.1,
                    sustain: 0.0,
                    release: 0.0,
                }),
            },
            ToneSpec {
                waveform: Waveform::Sine,
                frequency_relation: FrequencyRelation::Ratio(1.0),
                amplitude_envelope: Some(EnvelopeSpec::Adsr {
                    attack: 0.0,
                    decay: 0.0,
                    sustain: 1.0,
                    release: 0.0,
                }),
            },
        ],
        pitch_envelope: Some(EnvelopeSpec::Adsr {
            attack: 0.0,
            decay: 0.3,
            sustain: 0.5,
            release: 0.0,
        }),
        mix_mode: MixMode::Sum,
        amplitude_envelope: Some(EnvelopeSpec::Adsr {
            attack: 0.01,
            decay: 0.3,
            sustain: 0.0,
            release: 0.0,
        }),
        base_frequency: Some(58.0),
        fx: vec![],
        gain: 1.0,
        velocity_sensitivity: 0.0,
        mods: vec![],
    };

    let kick_system = compile_spec(&kick_spec, 44100.0);
    assert!(
        kick_system.is_ok(),
        "There should not be a compilation error with the kick specification"
    );

    let _kick_system = kick_system.unwrap();
}
