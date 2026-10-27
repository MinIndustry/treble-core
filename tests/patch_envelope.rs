use treble::{
    core::{Note, graph::System},
    patch::{ModuleRegistry, PatchRuntime, PatchSession, metadata::*},
};

fn document() -> InstrumentDocument {
    serde_json::from_value(serde_json::json!({
        "format_version":1, "instrument": {
            "name":"Envelope", "description":"", "voice":{"kind":"monophonic"},
            "patch":{"modules":[
                {"id":"env","type_id":"treble.adsr","type_version":1,"parameters":{
                    "attack":{"kind":"float","value":0.125}, "decay":{"kind":"float","value":0.125},
                    "sustain":{"kind":"float","value":0.5}, "release":{"kind":"float","value":0.125}}},
                {"id":"audio","type_id":"treble.control_to_audio","type_version":1,"parameters":{}}
            ],"cables":[{"id":"cv","from":{"module":"env","port":"out"},"to":{"module":"audio","port":"in"},"modulation":null}]},
            "interface":{"note_input":{"module":"env","port":"trigger"},"audio_outputs":[{"module":"audio","port":"out"}],"controls":[]}
        },"editor":null
    })).unwrap()
}
fn render(system: &mut System, frames: usize) -> Vec<[f32; 2]> {
    system.run_frames(frames);
    system.get_sink(0).unwrap().consume()
}
#[test]
fn adsr_notes_live_controls_release_and_panic_run_through_the_patch_session() {
    let registry = ModuleRegistry::builtins();
    let doc = document();
    let text = treble::patch::serialize_document(&doc).unwrap();
    let doc = treble::patch::parse_document(&text).unwrap();
    let mut session = PatchSession::new(registry, doc).unwrap();
    let prepared = session.snapshot().prepare(48000.0, 256).unwrap();
    let mut system = System::new();
    let mut runtime = PatchRuntime::default();
    runtime
        .apply(&mut system, session.commit(prepared).unwrap())
        .unwrap();
    assert!(render(&mut system, 256).iter().all(|f| *f == [0.0; 2]));
    let note = Note::from_midi(60);
    runtime
        .apply(&mut system, session.note_on(note, 1.0).unwrap())
        .unwrap();
    let attack = render(&mut system, 3000);
    assert!((attack.last().unwrap()[0] - 0.5).abs() < 1e-5);
    // Releasing during attack starts from 0.5, not the configured sustain.
    runtime
        .apply(&mut system, session.note_off(note).unwrap())
        .unwrap();
    let release = render(&mut system, 3000);
    assert!((release.last().unwrap()[0] - 0.25).abs() < 1e-5);
    assert!(release.windows(2).all(|w| w[1][0] <= w[0][0]));
    render(&mut system, 3000);
    assert!(render(&mut system, 256).iter().all(|f| *f == [0.0; 2]));
    for (name, value) in [("attack", 0.0), ("decay", 0.0), ("sustain", 0.8)] {
        let edit = session
            .set_parameter("env", name, ParameterValue::Float(value))
            .unwrap();
        let cmd = edit.command.expect("must be live");
        runtime.apply(&mut system, cmd).unwrap();
    }
    runtime
        .apply(&mut system, session.note_on(note, 1.0).unwrap())
        .unwrap();
    assert!(
        render(&mut system, 256)
            .iter()
            .all(|f| (f[0] - 0.8).abs() < 1e-6 && f[0] == f[1])
    );
    runtime
        .apply(&mut system, session.all_notes_off().unwrap())
        .unwrap();
    assert!(render(&mut system, 256).iter().all(|f| *f == [0.0; 2]));
}
#[test]
fn continuous_voice_runs_attack_to_sustain_without_a_note_input() {
    let registry = ModuleRegistry::builtins();
    let mut doc = document();
    doc.instrument.voice = VoiceConfiguration::Continuous;
    doc.instrument.interface.note_input = None;
    // Keep the envelope peak below the device sink limiter's threshold.
    doc.instrument.patch.modules.push(ModuleInstance {
        id: "gain".into(),
        type_id: "treble.gain".into(),
        type_version: 1,
        parameters: std::collections::BTreeMap::from([(
            "gain".into(),
            ParameterValue::Float(0.25),
        )]),
    });
    doc.instrument.patch.cables.push(Cable {
        id: "level".into(),
        from: Endpoint {
            module: "audio".into(),
            port: "out".into(),
        },
        to: Endpoint {
            module: "gain".into(),
            port: "in".into(),
        },
        modulation: None,
    });
    doc.instrument.interface.audio_outputs = vec![Endpoint {
        module: "gain".into(),
        port: "out".into(),
    }];
    let mut system = registry.prepare(&doc, 48000.0, 256).unwrap().into_system();
    render(&mut system, 12000);
    assert!(render(&mut system, 256).iter().all(|f| *f == [0.125; 2]));
}

#[test]
fn signal_gate_edges_use_current_samples_even_when_the_gate_processor_is_declared_later() {
    let registry = ModuleRegistry::builtins();
    let mut doc = document();
    doc.instrument.voice = VoiceConfiguration::Continuous;
    doc.instrument.interface.note_input = None;
    let envelope = &mut doc.instrument.patch.modules[0];
    for name in ["attack", "decay", "release"] {
        envelope
            .parameters
            .insert(name.into(), ParameterValue::Float(0.0));
    }
    envelope
        .parameters
        .insert("sustain".into(), ParameterValue::Float(0.25));
    doc.instrument.patch.modules.push(ModuleInstance {
        id: "lfo".into(),
        type_id: "treble.lfo".into(),
        type_version: 1,
        parameters: std::collections::BTreeMap::from([(
            "frequency".into(),
            ParameterValue::Float(1000.0),
        )]),
    });
    doc.instrument.patch.modules.push(ModuleInstance {
        id: "gate".into(),
        type_id: "treble.control_to_gate".into(),
        type_version: 1,
        parameters: Default::default(),
    });
    for (id, from, port, to, input) in [
        ("threshold", "lfo", "out", "gate", "in"),
        ("trigger", "gate", "out", "env", "gate"),
    ] {
        doc.instrument.patch.cables.push(Cable {
            id: id.into(),
            from: Endpoint {
                module: from.into(),
                port: port.into(),
            },
            to: Endpoint {
                module: to.into(),
                port: input.into(),
            },
            modulation: None,
        });
    }
    let make = || registry.prepare(&doc, 48000.0, 256).unwrap().into_system();
    let mut full = make();
    let expected = render(&mut full, 768);
    assert_eq!(expected[0], [0.0; 2]);
    assert_eq!(expected[1], [0.25; 2]);
    assert!(expected.contains(&[0.0; 2]));
    assert!(expected.iter().all(|v| *v == [0.0; 2] || *v == [0.25; 2]));
    let mut split = make();
    let mut actual = render(&mut split, 257);
    actual.extend(render(&mut split, 511));
    assert_eq!(expected, actual);
}

#[test]
fn killing_routed_notes_preserves_independent_constant_sources() {
    let mut doc = document();
    for name in ["attack", "decay"] {
        doc.instrument.patch.modules[0]
            .parameters
            .insert(name.into(), ParameterValue::Float(0.0));
    }
    doc.instrument.patch.modules.push(ModuleInstance {
        id: "dc".into(),
        type_id: "treble.constant".into(),
        type_version: 1,
        parameters: std::collections::BTreeMap::from([(
            "value".into(),
            ParameterValue::Float(0.1),
        )]),
    });
    doc.instrument.interface.audio_outputs.push(Endpoint {
        module: "dc".into(),
        port: "out".into(),
    });
    let mut session = PatchSession::new(ModuleRegistry::builtins(), doc).unwrap();
    let prepared = session.snapshot().prepare(48000.0, 256).unwrap();
    let mut system = System::silent();
    let mut runtime = PatchRuntime::default();
    runtime
        .apply(&mut system, session.commit(prepared).unwrap())
        .unwrap();
    runtime
        .apply(
            &mut system,
            session.note_on(Note::from_midi(60), 1.0).unwrap(),
        )
        .unwrap();
    runtime
        .apply(&mut system, session.kill_notes().unwrap())
        .unwrap();
    assert!(
        render(&mut system, 256)
            .iter()
            .all(|v| (v[0] - 0.1).abs() < 1e-6)
    );
    runtime
        .apply(&mut system, session.all_notes_off().unwrap())
        .unwrap();
    assert!(render(&mut system, 256).iter().all(|v| *v == [0.0; 2]));
}
