//! End-to-end references deliberately do not use the modulation composition
//! helper: drive a plain oscillator one sample at a time, then compare the rack.
use std::collections::BTreeMap;
use treble::{
    Note,
    core::{
        Block,
        generator::prelude::{SingleToneGenerator, builder::ToneGeneratorBuilder},
        graph::System,
    },
    patch::{ModuleRegistry, PatchRuntime, PatchSession, metadata::*},
};

const RATE: f32 = 48_000.0;
const BASE: f32 = 440.0;
const AMPLITUDE: f32 = 0.2;
const MOD_RATE: f32 = 750.0;

fn endpoint(module: &str, port: &str) -> Endpoint {
    Endpoint {
        module: module.into(),
        port: port.into(),
    }
}

fn document(parameter: &str, mode: ModulationMode, depth: f64) -> InstrumentDocument {
    InstrumentDocument {
        format_version: 1,
        instrument: InstrumentDefinition {
            name: "Modulation reference".into(),
            description: String::new(),
            patch: Patch {
                // Deliberately put the consumer before its modulator.
                modules: vec![
                    ModuleInstance {
                        id: "carrier".into(),
                        type_id: "treble.oscillator".into(),
                        type_version: 1,
                        parameters: BTreeMap::from([
                            ("frequency".into(), ParameterValue::Float(f64::from(BASE))),
                            (
                                "amplitude".into(),
                                ParameterValue::Float(f64::from(AMPLITUDE)),
                            ),
                        ]),
                    },
                    ModuleInstance {
                        id: "modulator".into(),
                        type_id: "treble.lfo".into(),
                        type_version: 1,
                        parameters: BTreeMap::from([(
                            "frequency".into(),
                            ParameterValue::Float(f64::from(MOD_RATE)),
                        )]),
                    },
                ],
                cables: vec![Cable {
                    id: "modulation".into(),
                    from: endpoint("modulator", "out"),
                    to: endpoint("carrier", parameter),
                    modulation: Some(ModulationSettings { mode, depth }),
                }],
            },
            interface: InstrumentInterface {
                note_input: Some(endpoint("carrier", "trigger")),
                // Also exercise the direct source-to-sink dispatch path.
                audio_outputs: vec![endpoint("carrier", "out")],
                controls: vec![],
            },
            voice: VoiceConfiguration::Continuous,
        },
        editor: None,
    }
}

fn render(system: &mut System, frames: usize) -> Block {
    system.run_frames(frames);
    system.get_sink(0).unwrap().consume()
}

fn prepared(document: &InstrumentDocument) -> System {
    ModuleRegistry::builtins()
        .prepare(document, RATE, 512)
        .unwrap()
        .into_system()
}

fn oscillator() -> SingleToneGenerator {
    let mut oscillator = ToneGeneratorBuilder::new().frequency(BASE).build();
    oscillator.start();
    oscillator
}

fn control_sample(phase: &mut f64) -> f32 {
    let value = (*phase * std::f64::consts::TAU).sin() as f32;
    *phase = (*phase + f64::from(MOD_RATE) / f64::from(RATE)).fract();
    value
}

fn reference_fm(
    oscillator: &mut SingleToneGenerator,
    phase: &mut f64,
    base: f32,
    depth: f64,
    frames: usize,
) -> Block {
    (0..frames)
        .map(|_| {
            let frequency = (f64::from(base) + depth * f64::from(control_sample(phase))) as f32;
            oscillator.set_frequency(frequency);
            [oscillator.tick(1.0 / RATE) * AMPLITUDE; 2]
        })
        .collect()
}

fn assert_samples(actual: &Block, expected: &Block) {
    assert_eq!(actual.len(), expected.len());
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        for channel in 0..2 {
            assert!(
                (actual[channel] - expected[channel]).abs() < 1e-6,
                "sample {index}, channel {channel}: {} != {}",
                actual[channel],
                expected[channel]
            );
        }
    }
}

#[test]
fn linear_fm_matches_per_sample_reference_despite_a_zero_mean_block() {
    let mut system = prepared(&document("frequency_cv", ModulationMode::Add, 120.0));
    let actual = render(&mut system, 1536);
    let expected = reference_fm(&mut oscillator(), &mut 0.0, BASE, 120.0, 1536);
    assert_samples(&actual, &expected);
    let mut plain = oscillator();
    let plain: Block = (0..1536)
        .map(|_| [plain.tick(1.0 / RATE) * AMPLITUDE; 2])
        .collect();
    assert!(
        actual
            .iter()
            .zip(plain)
            .any(|(a, b)| (a[0] - b[0]).abs() > 0.01),
        "a whole-block mean would incorrectly erase this modulation"
    );
}

#[test]
fn amplitude_modulation_matches_the_control_sample_at_each_audio_sample() {
    let mut system = prepared(&document("amplitude_cv", ModulationMode::Multiply, 0.5));
    let actual = render(&mut system, 1024);
    let mut plain = oscillator();
    let mut phase = 0.0;
    let expected: Block = (0..1024)
        .map(|_| {
            let gain =
                (f64::from(AMPLITUDE) * (1.0 + 0.5 * f64::from(control_sample(&mut phase)))) as f32;
            [plain.tick(1.0 / RATE) * gain; 2]
        })
        .collect();
    assert_samples(&actual, &expected);
}

#[test]
fn declaration_order_and_irregular_block_splits_do_not_change_fm_audio() {
    let doc = document("frequency_cv", ModulationMode::Add, 120.0);
    let expected = render(&mut prepared(&doc), 1536);
    let mut reversed = doc.clone();
    reversed.instrument.patch.modules.reverse();
    let mut system = prepared(&reversed);
    let actual: Block = [1, 7, 63, 129, 512, 824]
        .into_iter()
        .flat_map(|frames| render(&mut system, frames))
        .collect();
    assert_samples(&actual, &expected);
}

#[test]
fn note_pitch_and_live_fixed_base_edits_compose_without_restarting_modulation() {
    let mut doc = document("frequency_cv", ModulationMode::Add, 120.0);
    doc.instrument.voice = VoiceConfiguration::Monophonic;
    let mut session = PatchSession::new(ModuleRegistry::builtins(), doc).unwrap();
    let command = session
        .commit(session.snapshot().prepare(RATE, 512).unwrap())
        .unwrap();
    let mut runtime = PatchRuntime::default();
    let mut system = System::silent();
    runtime.apply(&mut system, command).unwrap();
    assert!(
        render(&mut system, 96)
            .iter()
            .all(|frame| *frame == [0.0; 2])
    );
    let note = Note::from_midi(81);
    runtime
        .apply(&mut system, session.note_on(note, 1.0).unwrap())
        .unwrap();
    let mut reference = oscillator();
    let mut phase = 0.0;
    for _ in 0..96 {
        control_sample(&mut phase);
    }
    assert_samples(
        &render(&mut system, 128),
        &reference_fm(&mut reference, &mut phase, note.frequency(), 120.0, 128),
    );
    let edit = session
        .set_parameter("carrier", "follow_keys", ParameterValue::Bool(false))
        .unwrap();
    runtime.apply(&mut system, edit.command.unwrap()).unwrap();
    assert_samples(
        &render(&mut system, 64),
        &reference_fm(&mut reference, &mut phase, BASE, 120.0, 64),
    );
    let edit = session
        .set_parameter("carrier", "frequency", ParameterValue::Float(660.0))
        .unwrap();
    runtime.apply(&mut system, edit.command.unwrap()).unwrap();
    assert_samples(
        &render(&mut system, 64),
        &reference_fm(&mut reference, &mut phase, 660.0, 120.0, 64),
    );
    let carrier = &session.document().instrument.patch.modules[0];
    assert_eq!(
        carrier.parameters["frequency"],
        ParameterValue::Float(660.0)
    );
}
