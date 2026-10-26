use std::collections::BTreeMap;
use treble::{
    core::{
        Block,
        generator::prelude::*,
        graph::{ModTarget, System},
    },
    patch::{ModuleRegistry, metadata::*, parse_document},
};

fn endpoint(module: &str, port: &str) -> Endpoint {
    Endpoint {
        module: module.into(),
        port: port.into(),
    }
}
fn module(id: &str, type_id: &str) -> ModuleInstance {
    ModuleInstance {
        id: id.into(),
        type_id: type_id.into(),
        type_version: 1,
        parameters: BTreeMap::new(),
    }
}
fn wire(id: &str, from: &str, to: &str, port: &str, mode: ModulationMode, depth: f64) -> Cable {
    Cable {
        id: id.into(),
        from: endpoint(from, "out"),
        to: endpoint(to, port),
        modulation: Some(ModulationSettings { mode, depth }),
    }
}
fn document() -> InstrumentDocument {
    let mut document = parse_document(include_str!("../examples/fixtures/rack.trbl")).unwrap();
    document
        .instrument
        .patch
        .modules
        .retain(|module| module.id != "filter");
    document
        .instrument
        .patch
        .modules
        .push(module("lfo", "treble.lfo"));
    document.instrument.patch.cables = vec![
        Cable {
            id: "audio".into(),
            from: endpoint("osc", "out"),
            to: endpoint("output", "in"),
            modulation: None,
        },
        wire(
            "fm",
            "lfo",
            "osc",
            "frequency_cv",
            ModulationMode::Add,
            100.0,
        ),
    ];
    document.instrument.voice = VoiceConfiguration::Continuous;
    document
}
fn prepare(document: &InstrumentDocument) -> System {
    ModuleRegistry::builtins()
        .prepare(document, 48000.0, 128)
        .unwrap()
        .into_system()
}
fn render(system: &mut System, frames: usize) -> Block {
    system.run_frames(frames);
    let mut output = system.get_sink(0).unwrap().consume();
    output.resize(frames, [0.0; 2]);
    output
}
fn close(actual: &Block, expected: &Block) {
    assert_eq!(actual.len(), expected.len());
    for (i, (a, b)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (a[0] - b[0]).abs() < 2e-5 && (a[1] - b[1]).abs() < 2e-5,
            "frame {i}: {a:?} != {b:?}"
        );
    }
}
fn lfo(frequency: f64, frames: usize) -> Vec<f32> {
    let mut phase: f64 = 0.0;
    (0..frames)
        .map(|_| {
            let value = (phase * std::f64::consts::TAU).sin() as f32;
            phase = (phase + frequency / 48000.0).fract();
            value
        })
        .collect()
}

#[test]
fn linear_fm_is_current_sample_and_independent_of_module_order_or_block_splits() {
    let mut document = document();
    let count = 2048;
    let mut reference = builder::ToneGeneratorBuilder::new().build();
    reference.start();
    let expected: Block = lfo(2.0, count)
        .into_iter()
        .map(|cv| {
            reference.set_frequency((440.0 + 100.0 * f64::from(cv)) as f32);
            [reference.tick(1.0 / 48000.0) * 0.2 * 0.8; 2]
        })
        .collect();
    let actual = render(&mut prepare(&document), count);
    close(&actual, &expected);
    document.instrument.patch.modules.reverse();
    close(&render(&mut prepare(&document), count), &expected);
    let mut split = prepare(&document);
    let segmented = [
        render(&mut split, 13),
        render(&mut split, 117),
        render(&mut split, count - 130),
    ]
    .concat();
    assert_eq!(actual, segmented);
}

#[test]
fn shared_modulator_fanout_renders_once_and_am_uses_each_sample() {
    let mut document = document();
    document.instrument.patch.cables.push(wire(
        "am",
        "lfo",
        "osc",
        "amplitude_cv",
        ModulationMode::Multiply,
        0.5,
    ));
    let mut one = prepare(&document);
    let baseline = render(&mut one, 2048);
    let mut reference = builder::ToneGeneratorBuilder::new().build();
    reference.start();
    let expected: Block = lfo(2.0, 2048)
        .into_iter()
        .map(|cv| {
            reference.set_frequency((440.0 + 100.0 * f64::from(cv)) as f32);
            let amplitude = (f64::from(0.2f32) * (1.0 + 0.5 * f64::from(cv))) as f32;
            [reference.tick(1.0 / 48000.0) * amplitude * 0.8; 2]
        })
        .collect();
    close(&baseline, &expected);
    document
        .instrument
        .patch
        .modules
        .push(module("second", "treble.oscillator"));
    document.instrument.patch.cables.push(wire(
        "fm2",
        "lfo",
        "second",
        "frequency_cv",
        ModulationMode::Add,
        100.0,
    ));
    document.instrument.patch.cables.push(wire(
        "am2",
        "lfo",
        "second",
        "amplitude_cv",
        ModulationMode::Multiply,
        0.5,
    ));
    document.instrument.patch.cables.push(Cable {
        id: "audio2".into(),
        from: endpoint("second", "out"),
        to: endpoint("output", "in"),
        modulation: None,
    });
    let doubled = render(&mut prepare(&document), 2048);
    let expected = baseline
        .into_iter()
        .map(|frame| [frame[0] * 2.0; 2])
        .collect();
    close(&doubled, &expected);
}

#[test]
fn chained_control_sources_render_in_dependency_order_without_a_block_of_latency() {
    let mut document = document();
    document
        .instrument
        .patch
        .modules
        .push(module("upstream", "treble.lfo"));
    document.instrument.patch.cables.push(wire(
        "chain",
        "upstream",
        "lfo",
        "frequency_cv",
        ModulationMode::Add,
        1.0,
    ));
    let mut phase: f64 = 0.0;
    let mut oscillator = builder::ToneGeneratorBuilder::new().build();
    oscillator.start();
    let expected: Block = lfo(2.0, 4096)
        .into_iter()
        .map(|upstream| {
            let cv = (phase * std::f64::consts::TAU).sin() as f32;
            let frequency = (2.0f64.log2() + f64::from(upstream)).exp2() as f32;
            phase = (phase + f64::from(frequency) / 48000.0).fract();
            oscillator.set_frequency((440.0 + 100.0 * f64::from(cv)) as f32);
            [oscillator.tick(1.0 / 48000.0) * 0.2 * 0.8; 2]
        })
        .collect();
    close(&render(&mut prepare(&document), 4096), &expected);
    document.instrument.patch.modules.reverse();
    close(&render(&mut prepare(&document), 4096), &expected);
}

#[test]
fn malformed_control_connections_and_combined_cycles_have_document_paths() {
    let registry = ModuleRegistry::builtins();
    let mut document = document();
    document.instrument.patch.cables[1].from = endpoint("osc", "out");
    assert!(
        registry
            .prepare(&document, 48000.0, 128)
            .unwrap_err()
            .to_string()
            .contains("cables[1]")
    );
    document = self::document();
    document.instrument.patch.cables[1].to.port = "missing_cv".into();
    assert!(
        registry
            .prepare(&document, 48000.0, 128)
            .unwrap_err()
            .to_string()
            .contains("cables[1].to.port")
    );
    document = self::document();
    document.instrument.patch.cables.push(wire(
        "cycle",
        "lfo",
        "lfo",
        "frequency_cv",
        ModulationMode::Add,
        1.0,
    ));
    let error = registry
        .prepare(&document, 48000.0, 128)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("cables[2]") && error.contains("cycle"),
        "{error}"
    );
}

#[test]
fn low_level_mutations_invalidate_safely_but_failed_edits_and_merges_do_not() {
    use petgraph::graph::NodeIndex;
    let document = document();
    let mut system = prepare(&document);
    let mut reference = system.clone();
    assert!(system.remove_source(usize::MAX).is_none());
    assert!(system.remove_sink(usize::MAX).is_none());
    assert!(system.remove_filter(NodeIndex::new(usize::MAX)).is_none());
    assert!(
        system
            .disconnect(NodeIndex::new(0), NodeIndex::new(999))
            .is_err()
    );
    assert!(
        system
            .add_mod_wire(0, ModTarget::Source(0), "frequency".into())
            .is_err()
    );
    let mut destination = System::new();
    assert!(destination.absorb_mapped(system.clone()).is_err());
    assert_eq!(destination.sources_len(), 0);
    assert!(system.absorb_mapped(System::new()).is_err());
    close(&render(&mut system, 128), &render(&mut reference, 128));
    system.remove_source(0).unwrap();
    assert!(system.compute().is_err());
    assert!(
        render(&mut system, 128)
            .iter()
            .all(|frame| *frame == [0.0; 2])
    );
}
