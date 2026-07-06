use treble::Note;
use treble::core::Block;
use treble::core::generator::prelude::Waveform;
use treble::core::graph::{MonophonicAllocationStrategy, System};
use treble::instruments::Instrument;
use treble::instruments::prelude::Kick;
use treble::instruments::prelude::{
    EnvelopeSpec, FxSpec, InstrumentRegistry, InstrumentSpec, SpecError, ToneSpec, VoiceSpec,
    compile_spec, validate_spec,
};

const SAMPLE_RATE: f32 = 44100.0;

/// Render `blocks` blocks from a system after a note-start, returning the
/// left-channel samples.
fn render(system: &mut System, blocks: usize) -> Vec<f32> {
    system.compute().expect("compute failed");
    system.start_note(0, Note::new(treble::NOTES::A, 4), 1.0);
    (0..blocks)
        .flat_map(|_| {
            system.run();
            system
                .get_sink(0)
                .map(|s| s.consume())
                .unwrap_or_else(|_| Block::new())
        })
        .map(|frame| frame[0])
        .collect()
}

fn rms(samples: &[f32]) -> f32 {
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len().max(1) as f32).sqrt()
}

#[test]
fn kick_spec_compiles() {
    let registry = InstrumentRegistry::built_in();
    let kick = registry.get("kick").expect("kick is a built-in");
    assert!(compile_spec(kick, SAMPLE_RATE).is_ok());
}

/// The registry kick must sound like the native `Kick` struct: compare
/// windowed RMS envelopes (sample-exact comparison is impossible — tone
/// generators randomize their start phase and white noise is unseeded).
#[test]
fn kick_spec_matches_native_kick() {
    let registry = InstrumentRegistry::built_in();
    let spec = registry.get("kick").expect("kick is a built-in");

    let mut native = Kick::new().as_system(SAMPLE_RATE);
    let mut from_spec = compile_spec(spec, SAMPLE_RATE).expect("kick spec compiles");

    // ~0.37s at the default block size — covers the full 0.3s decay.
    let native_samples = render(&mut native, 256);
    let spec_samples = render(&mut from_spec, 256);
    assert_eq!(native_samples.len(), spec_samples.len());

    let window = 2048;
    let mut compared = 0;
    for (i, (native_win, spec_win)) in native_samples
        .chunks(window)
        .zip(spec_samples.chunks(window))
        .enumerate()
    {
        let (native_rms, spec_rms) = (rms(native_win), rms(spec_win));
        // Skip windows where both are effectively silent.
        if native_rms < 1e-4 && spec_rms < 1e-4 {
            continue;
        }
        let relative = (native_rms - spec_rms).abs() / native_rms.max(spec_rms);
        assert!(
            relative < 0.25,
            "window {i}: native rms {native_rms}, spec rms {spec_rms} (relative diff {relative})"
        );
        compared += 1;
    }
    assert!(compared >= 4, "too few non-silent windows ({compared})");

    // Neither render may contain NaN (regression: zero-duration envelope
    // segments used to poison the oscillator phase).
    assert!(spec_samples.iter().all(|s| s.is_finite()));
}

#[test]
fn validate_rejects_unknown_filter() {
    let mut spec = minimal_spec();
    spec.fx.push(FxSpec {
        type_id: "NotAFilter".into(),
        params: Default::default(),
    });
    assert!(matches!(
        validate_spec(&spec),
        Err(SpecError::UnknownFilter(name)) if name == "NotAFilter"
    ));
}

#[test]
fn validate_rejects_unknown_parameter() {
    let mut spec = minimal_spec();
    spec.fx.push(FxSpec {
        type_id: "LowPassFilter".into(),
        params: [("bogus_param".to_string(), 1.0)].into(),
    });
    assert!(matches!(
        validate_spec(&spec),
        Err(SpecError::UnknownParameter { filter, param })
            if filter == "LowPassFilter" && param == "bogus_param"
    ));
}

/// Regression: the first fx-chain implementation never connected a
/// single-filter chain to the sink (and dropped inter-filter edges) — a
/// 1-fx and a 2-fx instrument must both produce sound.
#[test]
fn fx_chains_are_fully_connected() {
    for fx_count in [0usize, 1, 2] {
        let mut spec = minimal_spec();
        for _ in 0..fx_count {
            spec.fx.push(FxSpec {
                type_id: "LowPassFilter".into(),
                params: [("cutoff_frequency".to_string(), 8000.0)].into(),
            });
        }
        let mut system = compile_spec(&spec, SAMPLE_RATE).expect("spec compiles");
        let samples = render(&mut system, 32);
        assert!(
            rms(&samples) > 1e-4,
            "{fx_count}-fx chain produced silence — chain not connected to sink?"
        );
    }
}

/// A sine tone with a constant envelope: always audible right after start.
fn minimal_spec() -> InstrumentSpec {
    InstrumentSpec {
        name: "test-sine".into(),
        voice: VoiceSpec::Mono {
            track_pitch: true,
            allocation: MonophonicAllocationStrategy::Replace,
        },
        tones: vec![ToneSpec {
            waveform: Waveform::Sine,
            frequency_relation: Some(treble::core::generator::prelude::FrequencyRelation::Identity),
            frequency: None,
            amplitude_envelope: None,
        }],
        mix_mode: treble::core::generator::prelude::MixMode::Sum,
        pitch_envelope: None,
        amplitude_envelope: Some(EnvelopeSpec::Adsr {
            attack: 0.001,
            decay: 0.1,
            sustain: 0.8,
            release: 0.1,
        }),
        base_frequency: Some(440.0),
        fx: vec![],
        gain: 1.0,
        velocity_sensitivity: 1.0,
        mods: vec![],
    }
}
