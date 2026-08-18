use treble::core::generator::prelude::FrequencyRelation;
use treble::instruments::prelude::{InstrumentRegistry, compile_spec, validate_spec};
use treble::instruments::spec::{EnvelopeSpec, SegmentSpec};

const BUILT_INS: [&str; 15] = [
    "kick", "snare", "hihat", "clap", "rim", "tom", "sine", "saw", "square", "triangle", "piano",
    "bass", "pad", "pluck", "bell",
];

#[test]
fn built_in_registry_is_complete() {
    let registry = InstrumentRegistry::built_in();
    assert_eq!(registry.len(), BUILT_INS.len());
    for name in BUILT_INS {
        assert!(registry.get(name).is_some(), "missing built-in '{name}'");
    }
    assert!(registry.get("does-not-exist").is_none());
}

#[test]
fn every_built_in_validates_and_compiles() {
    let registry = InstrumentRegistry::built_in();
    for name in BUILT_INS {
        let spec = registry.get(name).unwrap();
        validate_spec(spec).unwrap_or_else(|e| panic!("'{name}' fails validation: {e}"));
        for sample_rate in [44100.0, 48000.0] {
            compile_spec(spec, sample_rate)
                .unwrap_or_else(|e| panic!("'{name}' fails to compile at {sample_rate}: {e}"));
        }
    }
}

#[test]
fn every_built_in_is_lossless_through_json() {
    let registry = InstrumentRegistry::built_in();
    for name in BUILT_INS {
        let original = registry.get(name).unwrap();
        let json = serde_json::to_string_pretty(original).unwrap();
        let decoded: treble::instruments::spec::InstrumentSpec = serde_json::from_str(&json)
            .unwrap_or_else(|error| panic!("'{name}' JSON failed: {error}"));
        assert_eq!(
            serde_json::to_value(original).unwrap(),
            serde_json::to_value(decoded).unwrap(),
            "'{name}' changed during JSON round-trip"
        );
    }
}

#[test]
fn percussion_classification_is_derived_from_voice() {
    let registry = InstrumentRegistry::built_in();
    for name in ["kick", "snare", "hihat", "clap", "rim", "tom"] {
        assert!(
            registry.is_percussion(name),
            "'{name}' should be percussion"
        );
    }
    for name in ["sine", "piano", "bass"] {
        assert!(!registry.is_percussion(name), "'{name}' is not percussion");
    }
    assert!(!registry.is_percussion("does-not-exist"));
}

#[test]
fn piano_spec_contains_balanced_harmonics() {
    let registry = InstrumentRegistry::built_in();
    let piano = registry.get("piano").unwrap();
    assert_eq!(piano.tones.len(), 4);
    let mut levels = Vec::new();
    for (index, tone) in piano.tones.iter().enumerate() {
        match (index, tone.frequency_relation.as_ref()) {
            (0, Some(FrequencyRelation::Identity))
            | (1, Some(FrequencyRelation::Harmonic(2)))
            | (2, Some(FrequencyRelation::Harmonic(3)))
            | (3, Some(FrequencyRelation::Harmonic(4))) => {}
            _ => panic!("unexpected piano partial at index {index}"),
        }
        let Some(EnvelopeSpec::Segment(SegmentSpec::Constant {
            value,
            duration: None,
        })) = &tone.amplitude_envelope
        else {
            panic!("piano partial must have a constant mix envelope")
        };
        levels.push(*value);
    }
    assert!(levels.windows(2).all(|pair| pair[0] > pair[1]));
}

#[test]
fn hihat_uses_noise_with_subtle_sine_shimmer() {
    let registry = InstrumentRegistry::built_in();
    let hihat = registry.get("hihat").unwrap();
    assert_eq!(hihat.tones.len(), 4);
    assert!(matches!(
        hihat.tones[0].waveform,
        treble::core::generator::prelude::Waveform::WhiteNoise
    ));
    assert!(hihat.tones[1..].iter().all(|tone| matches!(
        tone.waveform,
        treble::core::generator::prelude::Waveform::Sine
    )));
    assert!(hihat.tones.iter().all(|tone| !matches!(
        tone.waveform,
        treble::core::generator::prelude::Waveform::Square
            | treble::core::generator::prelude::Waveform::SquareRaw
    )));
}

#[test]
fn register_and_unregister() {
    let mut registry = InstrumentRegistry::built_in();
    let mut custom = registry.get("kick").unwrap().clone();
    custom.name = "my-kick".into();

    registry.register(custom).expect("valid spec registers");
    assert!(registry.get("my-kick").is_some());

    assert!(registry.unregister("my-kick"));
    assert!(!registry.unregister("my-kick"));
    assert!(registry.get("my-kick").is_none());
}
