use treble::instruments::prelude::{InstrumentRegistry, compile_spec, validate_spec};

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
