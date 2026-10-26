use std::collections::BTreeMap;
use treble::{
    Note,
    core::{
        filters::prelude::LowPassFilter,
        graph::{Entry, Filter},
    },
    patch::{PatchRuntime, PatchSession},
};
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

#[test]
fn filtered_audio_to_control_precedes_a_modulated_source_in_the_same_block() {
    let mut document = document();
    document.instrument.patch.modules.retain(|m| m.id != "lfo");
    let mut modulator = module("audio_mod", "treble.oscillator");
    modulator
        .parameters
        .insert("frequency".into(), ParameterValue::Float(80.0));
    modulator
        .parameters
        .insert("amplitude".into(), ParameterValue::Float(0.5));
    let mut filter = module("filter", "treble.low_pass");
    filter
        .parameters
        .insert("cutoff".into(), ParameterValue::Float(300.0));
    document.instrument.patch.modules.extend([
        module("convert", "treble.audio_to_control"),
        filter,
        modulator,
    ]);
    document.instrument.patch.cables = vec![
        Cable {
            id: "a".into(),
            from: endpoint("audio_mod", "out"),
            to: endpoint("filter", "in"),
            modulation: None,
        },
        Cable {
            id: "b".into(),
            from: endpoint("filter", "out"),
            to: endpoint("convert", "in"),
            modulation: None,
        },
        wire(
            "fm",
            "convert",
            "osc",
            "frequency_cv",
            ModulationMode::Add,
            300.0,
        ),
    ];
    document.instrument.interface.audio_outputs = vec![endpoint("osc", "out")];
    let count = 4096;
    let mut modulator = builder::ToneGeneratorBuilder::new().frequency(80.0).build();
    modulator.start();
    let input = (0..count)
        .map(|_| [modulator.tick(1.0 / 48000.0) * 0.5; 2])
        .collect();
    let mut filter = LowPassFilter::new(300.0, 48000.0);
    filter.push(std::sync::Arc::new(input), 0);
    let controls = filter.transform().remove(0);
    let mut carrier = builder::ToneGeneratorBuilder::new().build();
    carrier.start();
    let expected: Block = controls
        .into_iter()
        .map(|frame| {
            carrier.set_frequency((440.0 + 300.0 * f64::from(frame[0].clamp(-1.0, 1.0))) as f32);
            [carrier.tick(1.0 / 48000.0) * 0.2; 2]
        })
        .collect();
    close(&render(&mut prepare(&document), count), &expected);
    document.instrument.patch.modules.reverse();
    close(&render(&mut prepare(&document), count), &expected);
}

#[test]
fn filter_cutoff_resonance_and_output_gain_accept_sample_views() {
    let mut document = document();
    document
        .instrument
        .patch
        .modules
        .push(module("filter", "treble.low_pass"));
    document.instrument.patch.cables = vec![
        Cable {
            id: "a".into(),
            from: endpoint("osc", "out"),
            to: endpoint("filter", "in"),
            modulation: None,
        },
        Cable {
            id: "b".into(),
            from: endpoint("filter", "out"),
            to: endpoint("output", "in"),
            modulation: None,
        },
        wire(
            "cutoff",
            "lfo",
            "filter",
            "cutoff_cv",
            ModulationMode::Add,
            2.0,
        ),
        wire(
            "q",
            "lfo",
            "filter",
            "resonance_cv",
            ModulationMode::Add,
            0.5,
        ),
        wire(
            "gain",
            "lfo",
            "output",
            "level_cv",
            ModulationMode::Add,
            -0.5,
        ),
    ];
    let count = 4096;
    let mut carrier = builder::ToneGeneratorBuilder::new().build();
    carrier.start();
    let mut filter = LowPassFilter::new(1000.0, 48000.0);
    let mut expected = Vec::new();
    for cv in lfo(2.0, count) {
        let cutoff = (1000.0f64.log2() + 2.0 * f64::from(cv))
            .exp2()
            .clamp(1.0, 20000.0) as f32;
        let resonance = (f64::from(0.70710677f32) + 0.5 * f64::from(cv)).clamp(0.5, 20.0) as f32;
        filter.set_parameter("cutoff_frequency", cutoff);
        filter.set_parameter("resonance", resonance);
        filter.push(
            std::sync::Arc::new(vec![[carrier.tick(1.0 / 48000.0) * 0.2; 2]]),
            0,
        );
        let filtered = filter.transform()[0][0];
        let gain = (f64::from(0.8f32) - 0.5 * f64::from(cv)).clamp(0.0, 1.0) as f32;
        expected.push([filtered[0] * gain; 2]);
    }
    close(&render(&mut prepare(&document), count), &expected);
    let mut split = prepare(&document);
    close(
        &[
            render(&mut split, 1),
            render(&mut split, 17),
            render(&mut split, count - 18),
        ]
        .concat(),
        &expected,
    );
}

#[test]
fn multiwire_modes_follow_published_sum_formula_and_stable_cable_order() {
    for mode in [
        ModulationMode::Add,
        ModulationMode::Multiply,
        ModulationMode::Replace,
    ] {
        let mut document = document();
        let mut second = module("lfo2", "treble.lfo");
        second
            .parameters
            .insert("frequency".into(), ParameterValue::Float(3.0));
        document.instrument.patch.modules.push(second);
        let (first_depth, second_depth) = match mode {
            ModulationMode::Multiply => (0.5, -0.25),
            _ => (600.0, -200.0),
        };
        document.instrument.patch.cables[1].modulation = Some(ModulationSettings {
            mode: mode.clone(),
            depth: first_depth,
        });
        document.instrument.patch.cables.push(wire(
            "fm2",
            "lfo2",
            "osc",
            "frequency_cv",
            mode.clone(),
            second_depth,
        ));
        let count = 4096;
        let mut carrier = builder::ToneGeneratorBuilder::new().build();
        carrier.start();
        let expected = lfo(2.0, count)
            .into_iter()
            .zip(lfo(3.0, count))
            .map(|(a, b)| {
                let sum = first_depth * f64::from(a) + second_depth * f64::from(b);
                let frequency = match mode {
                    ModulationMode::Add => 440.0 + sum,
                    ModulationMode::Multiply => 440.0 * (1.0 + sum),
                    ModulationMode::Replace => sum,
                };
                carrier.set_frequency(frequency.clamp(1.0, 20000.0) as f32);
                [carrier.tick(1.0 / 48000.0) * 0.2 * 0.8; 2]
            })
            .collect();
        close(&render(&mut prepare(&document), count), &expected);
        document.instrument.patch.cables.reverse();
        document.instrument.patch.modules.reverse();
        close(&render(&mut prepare(&document), count), &expected);
    }
}

#[test]
fn monophonic_notes_use_effective_pitch_and_lfo_keeps_running_without_keys() {
    let mut document = document();
    document.instrument.voice = VoiceConfiguration::Monophonic;
    let mut session = PatchSession::new(ModuleRegistry::builtins(), document).unwrap();
    let mut runtime = PatchRuntime::default();
    let mut system = System::silent();
    let install = session
        .commit(session.snapshot().prepare(48000.0, 128).unwrap())
        .unwrap();
    runtime.apply(&mut system, install).unwrap();
    assert!(
        render(&mut system, 64)
            .iter()
            .all(|frame| *frame == [0.0; 2])
    );
    runtime
        .apply(
            &mut system,
            session.note_on(Note::from_midi(81), 0.5).unwrap(),
        )
        .unwrap();
    let controls = lfo(2.0, 64 + 512 + 512);
    let mut carrier = builder::ToneGeneratorBuilder::new().build();
    carrier.start();
    let expected = controls[64..576]
        .iter()
        .map(|cv| {
            carrier.set_frequency((880.0 + 100.0 * f64::from(*cv)) as f32);
            [carrier.tick(1.0 / 48000.0) * 0.2 * 0.5 * 0.8; 2]
        })
        .collect();
    close(&render(&mut system, 512), &expected);
    let fixed = session
        .set_parameter("osc", "follow_keys", ParameterValue::Bool(false))
        .unwrap();
    runtime.apply(&mut system, fixed.command.unwrap()).unwrap();
    let base = session
        .set_parameter("osc", "frequency", ParameterValue::Float(220.0))
        .unwrap();
    runtime.apply(&mut system, base.command.unwrap()).unwrap();
    let expected = controls[576..]
        .iter()
        .map(|cv| {
            carrier.set_frequency((220.0 + 100.0 * f64::from(*cv)) as f32);
            [carrier.tick(1.0 / 48000.0) * 0.2 * 0.5 * 0.8; 2]
        })
        .collect();
    close(&render(&mut system, 512), &expected);
    assert_eq!(
        session.document().instrument.patch.modules[0].parameters["frequency"],
        ParameterValue::Float(220.0)
    );
    runtime
        .apply(&mut system, session.all_notes_off().unwrap())
        .unwrap();
    assert!(
        render(&mut system, 128)
            .iter()
            .all(|frame| *frame == [0.0; 2])
    );
    let mut unplugged = session.document().clone();
    unplugged.instrument.patch.cables.retain(|c| c.id != "fm");
    session.replace_document(unplugged).unwrap();
    let install = session
        .commit(session.snapshot().prepare(48000.0, 128).unwrap())
        .unwrap();
    runtime.apply(&mut system, install).unwrap();
    runtime
        .apply(
            &mut system,
            session.note_on(Note::from_midi(81), 1.0).unwrap(),
        )
        .unwrap();
    let audio = render(&mut system, 512);
    let mut reference = builder::ToneGeneratorBuilder::new()
        .frequency(220.0)
        .build();
    reference.start();
    let expected = (0..512)
        .map(|_| [reference.tick(1.0 / 48000.0) * 0.2 * 0.8; 2])
        .collect();
    close(&audio, &expected);
}
