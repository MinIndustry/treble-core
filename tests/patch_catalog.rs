use std::{collections::BTreeMap, sync::Arc};
use treble::{
    core::{
        Block,
        filters::prelude::*,
        graph::{Filter, System},
    },
    patch::{ModuleRegistry, PatchRuntime, PatchSession, RuntimeModule, metadata::*},
};
fn module(id: &str, kind: &str, parameters: &[(&str, ParameterValue)]) -> ModuleInstance {
    ModuleInstance {
        id: id.into(),
        type_id: format!("treble.{kind}"),
        type_version: 1,
        parameters: parameters
            .iter()
            .map(|(k, v)| ((*k).into(), v.clone()))
            .collect(),
    }
}
fn f(value: f64) -> ParameterValue {
    ParameterValue::Float(value)
}
fn ep(module: &str, port: &str) -> Endpoint {
    Endpoint {
        module: module.into(),
        port: port.into(),
    }
}
fn wire(id: &str, from: (&str, &str), to: (&str, &str)) -> Cable {
    Cable {
        id: id.into(),
        from: ep(from.0, from.1),
        to: ep(to.0, to.1),
        modulation: None,
    }
}
fn document(
    modules: Vec<ModuleInstance>,
    cables: Vec<Cable>,
    output: (&str, &str),
) -> InstrumentDocument {
    InstrumentDocument {
        format_version: 1,
        instrument: InstrumentDefinition {
            name: "Catalog test".into(),
            description: String::new(),
            patch: Patch { modules, cables },
            interface: InstrumentInterface {
                note_input: None,
                audio_outputs: vec![ep(output.0, output.1)],
                controls: vec![],
            },
            voice: VoiceConfiguration::Continuous,
        },
        editor: None,
    }
}
fn render(document: &InstrumentDocument, count: usize) -> Block {
    let mut system = ModuleRegistry::builtins()
        .prepare(document, 48000.0, 256)
        .unwrap()
        .into_system();
    system.run_frames(count);
    system.get_sink(0).unwrap().consume()
}
#[test]
fn every_declared_live_control_is_received_and_rebuild_controls_stay_prepared() {
    let registry = ModuleRegistry::builtins();
    let modules: Vec<_> = registry
        .descriptors()
        .iter()
        .enumerate()
        .map(|(i, d)| {
            module(
                &format!("node{i}"),
                d.type_id.strip_prefix("treble.").unwrap(),
                &[],
            )
        })
        .collect();
    let controls: Vec<_> = registry
        .descriptors()
        .iter()
        .enumerate()
        .flat_map(|(i, d)| {
            d.parameters
                .iter()
                .map(move |p| (format!("node{i}"), p.clone()))
        })
        .collect();
    let mut doc = document(modules, vec![], ("node0", "out"));
    doc.instrument.interface.audio_outputs.clear();
    let mut session = PatchSession::new(registry, doc).unwrap();
    let mut runtime = PatchRuntime::default();
    let mut system = System::silent();
    let prepared = session.snapshot().prepare(48000.0, 256).unwrap();
    runtime
        .apply(&mut system, session.commit(prepared).unwrap())
        .unwrap();
    // Live first: rebuild edits intentionally invalidate subsequent live shortcuts.
    for (id, p) in controls
        .iter()
        .filter(|(_, p)| p.update == UpdateMode::Live)
    {
        let edit = session.set_parameter(id, &p.id, p.default.clone()).unwrap();
        let command = edit
            .command
            .unwrap_or_else(|| panic!("missing live mapping {id}.{}", p.id));
        runtime.apply(&mut system, command).unwrap();
    }
    for (id, p) in controls
        .iter()
        .filter(|(_, p)| p.update == UpdateMode::Rebuild)
    {
        assert!(
            session
                .set_parameter(id, &p.id, p.default.clone())
                .unwrap()
                .command
                .is_none()
        );
    }
}
#[test]
fn split_merge_and_exposed_nonzero_output_port_preserve_real_channel_identity() {
    for cv in [false, true] {
        let mut modules = vec![
            module("dc", "constant", &[("value", f(0.4))]),
            module("pan", "pan", &[("direction", f(-1.0))]),
            module("split", "stereo_split", &[]),
            module("merge", "stereo_merge", &[]),
        ];
        let mut cables = vec![
            wire("a", ("dc", "out"), ("pan", "in")),
            wire("b", ("pan", "out"), ("split", "in")),
            wire("c", ("split", "right"), ("merge", "left")),
            wire("d", ("split", "left"), ("merge", "right")),
        ];
        if cv {
            modules.push(module("lfo", "lfo", &[]));
            let mut c = wire("cv", ("lfo", "out"), ("pan", "direction_cv"));
            c.modulation = Some(ModulationSettings {
                mode: ModulationMode::Add,
                depth: 0.0,
            });
            cables.push(c);
        }
        let mut doc = document(modules, cables, ("merge", "out"));
        assert!(
            render(&doc, 64)
                .iter()
                .all(|frame| frame[0].abs() < 1e-6 && (frame[1] - 0.4).abs() < 1e-6)
        );
        doc.instrument.interface.audio_outputs = vec![ep("split", "right")];
        assert!(render(&doc, 64).iter().flatten().all(|v| v.abs() < 1e-6));
        doc.instrument.interface.audio_outputs = vec![ep("split", "left")];
        assert!(
            render(&doc, 64)
                .iter()
                .flatten()
                .all(|v| (*v - 0.4).abs() < 1e-6)
        );
    }
}
#[test]
fn control_streams_pass_through_math_in_the_current_block() {
    let doc = document(
        vec![
            module("multiply", "multiply", &[]),
            module("audio", "control_to_audio", &[]),
            module("lfo", "lfo", &[("frequency", f(750.0))]),
            module("dc", "constant", &[("value", f(-0.25))]),
        ],
        vec![
            wire("control", ("lfo", "out"), ("audio", "in")),
            wire("a", ("audio", "out"), ("multiply", "a")),
            wire("b", ("dc", "out"), ("multiply", "b")),
        ],
        ("multiply", "out"),
    );
    let samples = render(&doc, 512);
    for (i, frame) in samples.iter().enumerate() {
        let expected = (i as f64 / 64.0 * std::f64::consts::TAU).sin() as f32 * -0.25;
        assert!((frame[0] - expected).abs() < 1e-6, "sample {i}");
        assert_eq!(frame[0], frame[1]);
    }
}
#[test]
fn binary_port_b_is_not_redirected_to_a_and_missing_inputs_are_zero() {
    for (kind, expected) in [("add", 0.3), ("multiply", 0.0)] {
        let doc = document(
            vec![
                module("dc", "constant", &[("value", f(0.3))]),
                module("math", kind, &[]),
            ],
            vec![wire("b", ("dc", "out"), ("math", "b"))],
            ("math", "out"),
        );
        assert!(
            render(&doc, 32)
                .iter()
                .flatten()
                .all(|v| (*v - expected).abs() < 1e-6)
        );
    }
}
#[test]
fn typed_channel_mismatches_and_modulation_settings_on_streams_are_rejected() {
    let mut doc = document(
        vec![
            module("dc", "constant", &[]),
            module("merge", "stereo_merge", &[]),
        ],
        vec![wire("bad", ("dc", "out"), ("merge", "left"))],
        ("merge", "out"),
    );
    assert!(
        ModuleRegistry::builtins()
            .prepare(&doc, 48000.0, 64)
            .is_err()
    );
    doc.instrument.patch.modules = vec![
        module("lfo", "lfo", &[]),
        module("audio", "control_to_audio", &[]),
    ];
    doc.instrument.patch.cables = vec![wire("bad", ("lfo", "out"), ("audio", "in"))];
    doc.instrument.interface.audio_outputs = vec![ep("audio", "out")];
    doc.instrument.patch.cables[0].modulation = Some(ModulationSettings {
        mode: ModulationMode::Add,
        depth: 1.0,
    });
    assert!(
        ModuleRegistry::builtins()
            .prepare(&doc, 48000.0, 64)
            .is_err()
    );
}
#[test]
fn signed_gain_and_offset_receive_every_control_sample_without_overwriting_bases() {
    for (kind, parameter, port, base, depth) in [
        ("gain", "gain", "gain_cv", 1.0, 2.0),
        ("offset", "offset", "offset_cv", 0.1, 0.2),
    ] {
        let mut cv = wire("cv", ("lfo", "out"), ("processor", port));
        cv.modulation = Some(ModulationSettings {
            mode: ModulationMode::Add,
            depth,
        });
        let doc = document(
            vec![
                module("dc", "constant", &[("value", f(0.2))]),
                module("processor", kind, &[(parameter, f(base))]),
                module("lfo", "lfo", &[("frequency", f(750.0))]),
            ],
            vec![wire("audio", ("dc", "out"), ("processor", "in")), cv],
            ("processor", "out"),
        );
        let samples = render(&doc, 128);
        for (i, frame) in samples.iter().enumerate() {
            let signal = (i as f64 / 64.0 * std::f64::consts::TAU).sin() as f32;
            let value = (base + depth * f64::from(signal)) as f32;
            let expected = if kind == "gain" {
                0.2 * value
            } else {
                0.2 + value
            };
            assert!((frame[0] - expected).abs() < 1e-6);
        }
        assert_eq!(
            doc.instrument.patch.modules[1].parameters[parameter],
            f(base)
        );
    }
}
fn compare_filter(
    kind: &str,
    parameters: &[(&str, ParameterValue)],
    mut reference: Box<dyn Filter>,
    rate: f32,
) {
    let registry = ModuleRegistry::builtins();
    let RuntimeModule::Filter(mut actual) = registry
        .instantiate(&module("f", kind, parameters), rate)
        .unwrap()
    else {
        panic!("not a filter")
    };
    for segment in 0..16 {
        let block: Block = (0..256)
            .map(|i| {
                let t = (segment * 256 + i) as f32;
                [(t * 0.033).sin() * 0.6, (t * 0.091).cos() * 0.3]
            })
            .collect();
        actual.push(Arc::new(block.clone()), 0);
        reference.push(Arc::new(block), 0);
        let a = actual.transform();
        let b = reference.transform();
        assert_eq!(a.len(), b.len());
        for (got, want) in a
            .iter()
            .flatten()
            .flatten()
            .zip(b.iter().flatten().flatten())
        {
            assert!(
                got.is_finite() && (got - want).abs() < 1e-6,
                "{kind}: {got} != {want}"
            );
        }
    }
}
#[test]
fn registered_filters_match_existing_dsp_and_host_sample_rates() {
    for rate in [22050.0, 48000.0, 96000.0] {
        compare_filter(
            "high_pass",
            &[("cutoff", f(800.0)), ("resonance", f(1.2))],
            Box::new(HighPassFilter::with_resonance(800.0, 1.2, rate)),
            rate,
        );
        compare_filter(
            "band_pass",
            &[("low", f(400.0)), ("high", f(4000.0))],
            Box::new(BandPass::new(400.0, 4000.0, rate)),
            rate,
        );
        compare_filter(
            "resonant_band_pass",
            &[("center_frequency", f(1500.0)), ("quality", f(2.0))],
            Box::new(ResonantBandpassFilter::new(1500.0, 2.0, rate)),
            rate,
        );
        compare_filter(
            "moving_average",
            &[("size", ParameterValue::Int(5))],
            Box::new(MovingAverage::new(5)),
            rate,
        );
        compare_filter(
            "reverb",
            &[("amount", f(0.4))],
            Box::new(ReverbFilter::new(rate, 0.4)),
            rate,
        );
        compare_filter(
            "pan",
            &[("direction", f(-0.3))],
            Box::new(PanFilter::new(-0.3)),
            rate,
        );
        compare_filter(
            "auto_pan",
            &[
                ("frequency", f(3.0)),
                ("depth", f(0.8)),
                ("waveform", ParameterValue::Enum("triangle".into())),
            ],
            Box::new(AutoPanFilter::new(3.0, 0.8, PanWave::Triangle, rate)),
            rate,
        );
        compare_filter(
            "tremolo",
            &[("frequency", f(4.0)), ("depth", f(0.6))],
            Box::new(Tremolo::new(4.0, 0.6, rate)),
            rate,
        );
        compare_filter(
            "clipper",
            &[("maximum", f(0.25))],
            Box::new(Clipper::new(0.25)),
            rate,
        );
        for kind in ["compressor", "limiter"] {
            let mut reference: Box<dyn Filter> = if kind == "compressor" {
                Box::<Compressor>::default()
            } else {
                Box::<Limiter>::default()
            };
            reference.set_parameter("sample_rate", rate);
            compare_filter(kind, &[], reference, rate);
        }
    }
}
#[test]
fn all_catalog_defaults_validate_and_unknown_controls_fail_without_panicking() {
    let registry = ModuleRegistry::builtins();
    assert!(registry.descriptors().len() >= 32);
    for descriptor in registry.descriptors() {
        descriptor.validate().unwrap();
        let instance = ModuleInstance {
            id: "test".into(),
            type_id: descriptor.type_id.clone(),
            type_version: descriptor.version,
            parameters: descriptor
                .parameters
                .iter()
                .map(|p| (p.id.clone(), p.default.clone()))
                .collect(),
        };
        registry.instantiate(&instance, 44100.0).unwrap();
        let mut invalid = instance;
        invalid.parameters = BTreeMap::from([("unknown".into(), f(1.0))]);
        assert!(registry.instantiate(&invalid, 44100.0).is_err());
    }
}

#[test]
fn high_pass_and_pan_follow_sample_control_values() {
    for kind in ["high_pass", "pan"] {
        let (parameter, port, base, depth) = if kind == "high_pass" {
            ("cutoff", "cutoff_cv", 1000.0, 1.0)
        } else {
            ("direction", "direction_cv", 0.0, 0.8)
        };
        let mut cv = wire("cv", ("lfo", "out"), ("processor", port));
        cv.modulation = Some(ModulationSettings {
            mode: ModulationMode::Add,
            depth,
        });
        let doc = document(
            vec![
                module("dc", "constant", &[("value", f(0.4))]),
                module("processor", kind, &[(parameter, f(base))]),
                module("lfo", "lfo", &[("frequency", f(750.0))]),
            ],
            vec![wire("audio", ("dc", "out"), ("processor", "in")), cv],
            ("processor", "out"),
        );
        let actual = render(&doc, 256);
        let mut reference: Box<dyn Filter> = if kind == "high_pass" {
            Box::new(HighPassFilter::with_resonance(1000.0, 0.70710677, 48000.0))
        } else {
            Box::new(PanFilter::new(0.0))
        };
        for (i, got) in actual.iter().enumerate() {
            let signal = (i as f64 / 64.0 * std::f64::consts::TAU).sin() as f32;
            let value = if kind == "high_pass" {
                (base.log2() + depth * f64::from(signal)).exp2() as f32
            } else {
                (base + depth * f64::from(signal)) as f32
            };
            reference.set_parameter(
                if kind == "high_pass" {
                    "cutoff_frequency"
                } else {
                    "direction"
                },
                value,
            );
            reference.push(Arc::new(vec![[0.4; 2]]), 0);
            let want = reference.transform()[0][0];
            assert!(
                (got[0] - want[0]).abs() < 1e-6 && (got[1] - want[1]).abs() < 1e-6,
                "{kind} sample {i}"
            );
        }
    }
}
#[test]
fn sample_clamp_bounds_can_cross_without_a_panic_or_base_mutation() {
    let mut low = wire("lo", ("lfo", "out"), ("clamp", "minimum_cv"));
    low.modulation = Some(ModulationSettings {
        mode: ModulationMode::Add,
        depth: 2.0,
    });
    let mut high = wire("hi", ("lfo", "out"), ("clamp", "maximum_cv"));
    high.modulation = Some(ModulationSettings {
        mode: ModulationMode::Add,
        depth: -2.0,
    });
    let doc = document(
        vec![
            module("dc", "constant", &[("value", f(0.4))]),
            module(
                "clamp",
                "clamp",
                &[("minimum", f(-0.5)), ("maximum", f(0.5))],
            ),
            module("lfo", "lfo", &[("frequency", f(750.0))]),
        ],
        vec![wire("audio", ("dc", "out"), ("clamp", "in")), low, high],
        ("clamp", "out"),
    );
    for (i, frame) in render(&doc, 256).iter().enumerate() {
        let wave = (i as f64 / 64.0 * std::f64::consts::TAU).sin() as f32;
        let a = (-0.5 + 2.0 * f64::from(wave)) as f32;
        let b = (0.5 - 2.0 * f64::from(wave)) as f32;
        let expected = 0.4_f32.clamp(a.min(b), a.max(b));
        assert!((frame[0] - expected).abs() < 1e-6);
    }
    assert_eq!(
        doc.instrument.patch.modules[1].parameters["minimum"],
        f(-0.5)
    );
}
