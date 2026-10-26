use std::collections::BTreeMap;
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
