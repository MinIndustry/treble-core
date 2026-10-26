use std::collections::BTreeMap;
use treble::{
    Note,
    patch::{PatchApply, PatchRuntime, PatchSession},
};
use treble::{
    core::{Block, graph::System},
    patch::{ModuleRegistry, metadata::*},
};

fn instance(id: &str, type_id: &str) -> ModuleInstance {
    ModuleInstance {
        id: id.into(),
        type_id: type_id.into(),
        type_version: 1,
        parameters: BTreeMap::new(),
    }
}
fn endpoint(module: &str, port: &str) -> Endpoint {
    Endpoint {
        module: module.into(),
        port: port.into(),
    }
}
fn cable(id: &str, from: &str, to: &str) -> Cable {
    Cable {
        id: id.into(),
        from: endpoint(from, "out"),
        to: endpoint(to, "in"),
        modulation: None,
    }
}
fn document() -> InstrumentDocument {
    InstrumentDocument {
        format_version: 1,
        instrument: InstrumentDefinition {
            name: "Test".into(),
            description: String::new(),
            patch: Patch {
                modules: vec![
                    instance("osc", "treble.oscillator"),
                    instance("filter", "treble.low_pass"),
                    instance("output", "treble.output"),
                ],
                cables: vec![cable("a", "osc", "filter"), cable("b", "filter", "output")],
            },
            interface: InstrumentInterface {
                note_input: Some(endpoint("osc", "trigger")),
                audio_outputs: vec![endpoint("output", "out")],
                controls: vec![],
            },
            voice: VoiceConfiguration::Continuous,
        },
        editor: Some(serde_json::json!({"rack": {"osc": [3, 4]}, "monitors": [{"tap": "output"}]})),
    }
}
fn render(system: &mut System, frames: usize) -> Block {
    system.run_frames(frames);
    system.get_sink(0).unwrap().consume()
}
fn energy(block: &Block) -> f32 {
    block.iter().map(|f| f[0] * f[0]).sum()
}

fn playable_session() -> (PatchSession, PatchRuntime, System) {
    let mut doc = document();
    doc.instrument.voice = VoiceConfiguration::Monophonic;
    doc.instrument.patch.cables = vec![cable("direct", "osc", "output")];
    let mut session = PatchSession::new(ModuleRegistry::builtins(), doc).unwrap();
    let prepared = session.snapshot().prepare(48000.0, 128).unwrap();
    let command = session.commit(prepared).unwrap();
    let mut runtime = PatchRuntime::default();
    let mut system = System::silent();
    assert_eq!(
        runtime.apply(&mut system, command).unwrap(),
        PatchApply::Installed
    );
    (session, runtime, system)
}

fn pitch(block: &Block, rate: f32) -> f32 {
    let rising: Vec<_> = block
        .windows(2)
        .enumerate()
        .filter_map(|(i, w)| (w[0][0] <= 0.0 && w[1][0] > 0.0).then_some(i))
        .collect();
    rate * (rising.len() - 1) as f32 / (rising.last().unwrap() - rising[0]) as f32
}

#[test]
fn note_trigger_follow_keys_fixed_pitch_and_release() {
    let (mut session, mut runtime, mut system) = playable_session();
    let a4 = Note::from_midi(69);
    let a5 = Note::from_midi(81);
    assert_eq!(energy(&render(&mut system, 128)), 0.0);
    runtime
        .apply(&mut system, session.note_on(a4, 1.0).unwrap())
        .unwrap();
    assert!((pitch(&render(&mut system, 12000), 48000.0) - 440.0).abs() < 1.0);
    runtime
        .apply(&mut system, session.note_on(a5, 0.5).unwrap())
        .unwrap();
    assert!((pitch(&render(&mut system, 12000), 48000.0) - 880.0).abs() < 1.0);
    runtime
        .apply(&mut system, session.note_off(a5).unwrap())
        .unwrap();
    assert!((pitch(&render(&mut system, 12000), 48000.0) - 440.0).abs() < 1.0);
    let edit = session
        .set_parameter("osc", "follow_keys", ParameterValue::Bool(false))
        .unwrap();
    assert_eq!(edit.update, UpdateMode::Live);
    runtime.apply(&mut system, edit.command.unwrap()).unwrap();
    let edit = session
        .set_parameter("osc", "frequency", ParameterValue::Float(220.0))
        .unwrap();
    runtime.apply(&mut system, edit.command.unwrap()).unwrap();
    runtime
        .apply(&mut system, session.note_on(a5, 1.0).unwrap())
        .unwrap();
    assert!((pitch(&render(&mut system, 12000), 48000.0) - 220.0).abs() < 1.0);
    runtime
        .apply(&mut system, session.note_off(a4).unwrap())
        .unwrap();
    assert!(
        energy(&render(&mut system, 128)) > 0.0,
        "releasing an older key must not stop the held key"
    );
    runtime
        .apply(&mut system, session.note_off(a5).unwrap())
        .unwrap();
    assert_eq!(energy(&render(&mut system, 128)), 0.0);
    assert!(session.note_on(a4, f32::NAN).is_err());
    runtime
        .apply(&mut system, session.note_on(a4, 1.0).unwrap())
        .unwrap();
    runtime
        .apply(&mut system, session.all_notes_off().unwrap())
        .unwrap();
    assert_eq!(energy(&render(&mut system, 128)), 0.0);
}

#[test]
fn stale_preparations_and_out_of_order_installs_cannot_replace_latest_graph() {
    let (mut session, mut runtime, mut system) = playable_session();
    let old_preparation = session.snapshot().prepare(48000.0, 128).unwrap();
    session
        .set_parameter("osc", "frequency", ParameterValue::Float(330.0))
        .unwrap();
    assert!(session.commit(old_preparation).is_err());
    let old = session
        .commit(session.snapshot().prepare(48000.0, 128).unwrap())
        .unwrap();
    session
        .set_parameter("osc", "frequency", ParameterValue::Float(660.0))
        .unwrap();
    let latest = session
        .commit(session.snapshot().prepare(48000.0, 128).unwrap())
        .unwrap();
    assert_eq!(
        runtime.apply(&mut system, latest).unwrap(),
        PatchApply::Installed
    );
    assert_eq!(
        runtime.apply(&mut system, old).unwrap(),
        PatchApply::Superseded
    );
    runtime
        .apply(
            &mut system,
            session.note_on(Note::from_midi(69), 1.0).unwrap(),
        )
        .unwrap();
    let mut bad = session.document().clone();
    bad.instrument.patch.modules[0].type_id = "unavailable.plugin".into();
    session.replace_document(bad).unwrap();
    assert!(session.snapshot().prepare(48000.0, 128).is_err());
    assert!(
        energy(&render(&mut system, 1024)) > 1.0,
        "invalid draft leaves last installed graph playing"
    );
}

#[test]
fn live_base_values_persist_and_rebuild_edits_do_not_leak_to_playing_graph() {
    let (mut session, mut runtime, mut system) = playable_session();
    let old = session
        .set_parameter("osc", "amplitude", ParameterValue::Float(0.1))
        .unwrap()
        .command
        .unwrap();
    let new = session
        .set_parameter("osc", "amplitude", ParameterValue::Float(0.4))
        .unwrap()
        .command
        .unwrap();
    runtime.apply(&mut system, new).unwrap();
    assert_eq!(
        runtime.apply(&mut system, old).unwrap(),
        PatchApply::Superseded
    );
    let rebuild = session
        .set_parameter("osc", "waveform", ParameterValue::Enum("square".into()))
        .unwrap();
    assert_eq!(rebuild.update, UpdateMode::Rebuild);
    assert!(rebuild.command.is_none());
    assert!(
        session
            .set_parameter("osc", "amplitude", ParameterValue::Float(0.3))
            .unwrap()
            .command
            .is_none()
    );
    let loaded: InstrumentDocument =
        serde_json::from_str(&serde_json::to_string(session.document()).unwrap()).unwrap();
    assert_eq!(
        loaded.instrument.patch.modules[0].parameters["amplitude"],
        ParameterValue::Float(0.3)
    );
    let command = session
        .commit(session.snapshot().prepare(48000.0, 128).unwrap())
        .unwrap();
    runtime.apply(&mut system, command).unwrap();
    runtime
        .apply(
            &mut system,
            session.note_on(Note::from_midi(69), 1.0).unwrap(),
        )
        .unwrap();
    let audio = render(&mut system, 4096);
    assert!(audio.iter().any(|frame| frame[0].abs() > 0.23));
    assert!(audio.iter().all(|frame| frame[0].abs() < 0.26));
}

#[test]
fn structural_install_explicitly_resets_held_notes_and_state() {
    let (mut session, mut runtime, mut system) = playable_session();
    runtime
        .apply(
            &mut system,
            session.note_on(Note::from_midi(69), 1.0).unwrap(),
        )
        .unwrap();
    assert!(energy(&render(&mut system, 128)) > 0.0);
    session
        .replace_document(session.document().clone())
        .unwrap();
    let command = session
        .commit(session.snapshot().prepare(48000.0, 128).unwrap())
        .unwrap();
    runtime.apply(&mut system, command).unwrap();
    assert_eq!(energy(&render(&mut system, 128)), 0.0);
}

#[test]
fn a_note_input_bus_fans_out_to_oscillator_triggers_and_can_be_unplugged() {
    let mut doc = document();
    doc.instrument.voice = VoiceConfiguration::Monophonic;
    doc.instrument
        .patch
        .modules
        .push(instance("keyboard", "treble.note_input"));
    doc.instrument
        .patch
        .modules
        .push(instance("osc2", "treble.oscillator"));
    doc.instrument.interface.note_input = Some(endpoint("keyboard", "in"));
    doc.instrument.patch.cables.push(Cable {
        id: "note1".into(),
        from: endpoint("keyboard", "out"),
        to: endpoint("osc", "trigger"),
        modulation: None,
    });
    doc.instrument.patch.cables.push(Cable {
        id: "note2".into(),
        from: endpoint("keyboard", "out"),
        to: endpoint("osc2", "trigger"),
        modulation: None,
    });
    doc.instrument
        .patch
        .cables
        .push(cable("audio2", "osc2", "output"));
    let mut session = PatchSession::new(ModuleRegistry::builtins(), doc).unwrap();
    let mut runtime = PatchRuntime::default();
    let mut system = System::silent();
    let command = session
        .commit(session.snapshot().prepare(48000.0, 128).unwrap())
        .unwrap();
    runtime.apply(&mut system, command).unwrap();
    runtime
        .apply(
            &mut system,
            session.note_on(Note::from_midi(69), 1.0).unwrap(),
        )
        .unwrap();
    assert!(system.is_source_active(0));
    assert!(system.is_source_active(1));
    assert!(energy(&render(&mut system, 1024)) > 1.0);
    runtime
        .apply(&mut system, session.note_off(Note::from_midi(69)).unwrap())
        .unwrap();
    assert!(!system.is_source_active(0));
    assert!(!system.is_source_active(1));
    let mut unplugged = session.document().clone();
    unplugged
        .instrument
        .patch
        .cables
        .retain(|c| c.id != "note2");
    session.replace_document(unplugged).unwrap();
    let command = session
        .commit(session.snapshot().prepare(48000.0, 128).unwrap())
        .unwrap();
    runtime.apply(&mut system, command).unwrap();
    runtime
        .apply(
            &mut system,
            session.note_on(Note::from_midi(69), 1.0).unwrap(),
        )
        .unwrap();
    assert!(system.is_source_active(0));
    assert!(!system.is_source_active(1));
}

#[test]
fn document_roundtrip_compiles_and_renders_real_stereo_audio() {
    let document = document();
    let json = serde_json::to_string_pretty(&document).unwrap();
    let loaded: InstrumentDocument = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded, document);
    let registry = ModuleRegistry::builtins();
    let mut system = registry
        .prepare(&loaded, 48000.0, 128)
        .unwrap()
        .into_system();
    let block = render(&mut system, 4096);
    assert_eq!(block.len(), 4096);
    assert!(energy(&block) > 1.0);
    assert!(block.iter().all(|f| f[0] == f[1] && f[0].is_finite()));
}

#[test]
fn saving_an_incomplete_or_unavailable_instrument_is_independent_of_playback() {
    use treble::patch::{parse_document, serialize_document};
    let mut doc = document();
    doc.instrument.patch.modules[0].type_id = "plugin.not_installed".into();
    doc.instrument.patch.cables.clear();
    let saved = serialize_document(&doc).unwrap();
    assert_eq!(parse_document(&saved).unwrap(), doc);
    assert!(
        ModuleRegistry::builtins()
            .prepare(&doc, 48000.0, 128)
            .is_err()
    );
    doc.format_version = 999;
    assert!(
        serialize_document(&doc)
            .unwrap_err()
            .to_string()
            .contains("format_version")
    );
    assert!(
        parse_document("{")
            .unwrap_err()
            .to_string()
            .contains("JSON")
    );
}

#[test]
fn unlinking_by_cable_id_and_repreparing_silences_output() {
    let registry = ModuleRegistry::builtins();
    let mut doc = document();
    doc.instrument.patch.cables.retain(|c| c.id != "a");
    doc.validate().unwrap();
    let mut system = registry.prepare(&doc, 48000.0, 128).unwrap().into_system();
    assert_eq!(energy(&render(&mut system, 1024)), 0.0);
}

#[test]
fn fanout_and_input_sum_are_not_last_connection_wins() {
    let registry = ModuleRegistry::builtins();
    let mut doc = document();
    doc.instrument.patch.cables = vec![cable("a", "osc", "output")];
    let mut one = registry.prepare(&doc, 48000.0, 128).unwrap().into_system();
    let reference = render(&mut one, 1024);
    let mut parallel = instance("parallel", "treble.output");
    parallel
        .parameters
        .insert("level".into(), ParameterValue::Float(1.0));
    doc.instrument.patch.modules.push(parallel);
    doc.instrument
        .patch
        .cables
        .push(cable("b", "osc", "parallel"));
    doc.instrument
        .patch
        .cables
        .push(cable("c", "parallel", "output"));
    let mut two = registry.prepare(&doc, 48000.0, 128).unwrap().into_system();
    let doubled = render(&mut two, 1024);
    for (a, b) in reference.iter().zip(doubled) {
        assert!((2.0 * a[0] - b[0]).abs() < 1e-5);
    }
}

#[test]
fn malformed_and_unsupported_patches_report_paths_without_panics() {
    let registry = ModuleRegistry::builtins();
    let mut bad = document();
    bad.instrument.patch.cables[0].to.port = "missing".into();
    let errors = registry.prepare(&bad, 48000.0, 128).unwrap_err();
    assert!(errors.to_string().contains("cables[0].to.port"), "{errors}");
    bad = document();
    bad.instrument.patch.modules[0]
        .parameters
        .insert("frequency".into(), ParameterValue::Bool(false));
    assert!(
        registry
            .prepare(&bad, 48000.0, 128)
            .unwrap_err()
            .to_string()
            .contains("frequency")
    );
    bad = document();
    bad.instrument
        .patch
        .cables
        .push(cable("cycle", "output", "filter"));
    assert!(
        registry
            .prepare(&bad, 48000.0, 128)
            .unwrap_err()
            .to_string()
            .contains("cycle")
    );
    bad = document();
    bad.instrument.voice = VoiceConfiguration::Polyphonic { max_voices: 8 };
    assert!(
        registry
            .prepare(&bad, 48000.0, 128)
            .unwrap_err()
            .to_string()
            .contains("instrument.voice")
    );
    bad = document();
    bad.instrument.patch.modules[0].type_version = 99;
    assert!(registry.prepare(&bad, 48000.0, 128).is_err());
}
