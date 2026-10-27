//! Note-driven linear ADSR producing a sample-rate 0..1 control signal.
use crate::core::{
    Block, Note,
    graph::{Source, SourceInput},
};
use treble_derive::{ModuleMetadata, ModuleParameters, ParameterEnum};
use treble_meta::ParameterValue;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Stage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Clone, Debug, PartialEq, ParameterEnum)]
enum EnvelopeMode {
    #[treble(value(id = "gate"))]
    Gate,
    #[treble(value(id = "one_shot"))]
    OneShot,
    #[treble(value(id = "loop"))]
    Loop,
}
#[derive(Clone, Debug, PartialEq, ParameterEnum)]
enum RetriggerMode {
    #[treble(value(id = "current"))]
    Current,
    #[treble(value(id = "reset"))]
    Reset,
    #[treble(value(id = "legato"))]
    Legato,
}

#[derive(Clone, Debug, ModuleMetadata, ModuleParameters)]
#[treble(module(id = "treble.adsr", version = 1, name = "ADSR", category = "Envelopes"))]
#[treble(output(id = "out", index = 0, signal = control(domain = bipolar, rate = sample)))]
pub(super) struct Adsr {
    #[treble(param(id = "mode", default = EnvelopeMode::Gate, update = rebuild))]
    mode: EnvelopeMode,
    #[treble(param(id = "retrigger", default = RetriggerMode::Current, update = rebuild))]
    retrigger: RetriggerMode,
    #[treble(input(id = "gate", index = 1, signal = gate))]
    _gate: (),
    #[treble(input(id = "retrigger", index = 2, signal = gate))]
    _retrigger: (),
    #[treble(param(id = "attack", default = 0.01, range(min = 0.0, max = 30.0), unit = seconds))]
    attack: f32,
    #[treble(param(id = "decay", default = 0.1, range(min = 0.0, max = 30.0), unit = seconds))]
    decay: f32,
    #[treble(param(id = "sustain", default = 0.7, range(min = 0.0, max = 1.0)))]
    sustain: f32,
    #[treble(param(id = "release", default = 0.2, range(min = 0.0, max = 30.0), unit = seconds))]
    release: f32,
    #[treble(input(id = "trigger", index = 0, signal = note))]
    _trigger: (),
    #[treble(context(sample_rate))]
    sample_rate: f32,
    stage: Stage,
    level: f64,
    start_level: f64,
    progress: f64,
    held: Vec<Note>,
    drone: bool,
    gate_high: bool,
    retrigger_high: bool,
}
impl Adsr {
    pub(super) fn new(sample_rate: f32) -> Self {
        Self {
            mode: EnvelopeMode::Gate,
            retrigger: RetriggerMode::Current,
            _gate: (),
            _retrigger: (),
            gate_high: false,
            retrigger_high: false,
            attack: 0.01,
            decay: 0.1,
            sustain: 0.7,
            release: 0.2,
            _trigger: (),
            sample_rate,
            stage: Stage::Idle,
            level: 0.0,
            start_level: 0.0,
            progress: 0.0,
            held: vec![],
            drone: false,
        }
    }
    fn enter(&mut self, stage: Stage) {
        self.stage = stage;
        self.start_level = self.level;
        self.progress = 0.0;
    }
    fn gated(&self) -> bool {
        self.drone || self.gate_high || !self.held.is_empty()
    }
    fn fire(&mut self) {
        if self.retrigger == RetriggerMode::Legato && self.is_active() {
            return;
        }
        if self.retrigger == RetriggerMode::Reset {
            self.level = 0.0;
        }
        self.enter(Stage::Attack);
    }
    fn release_gate(&mut self) {
        if self.mode != EnvelopeMode::OneShot
            && !self.gated()
            && self.is_active()
            && self.stage != Stage::Release
        {
            self.enter(Stage::Release);
        }
    }
    fn render(&mut self, frames: usize, inputs: &[SourceInput]) -> Block {
        if inputs.iter().any(|input| input.port == 1) && self.drone {
            self.drone = false;
            if !self.gated() && self.level == 0.0 {
                self.enter(Stage::Idle);
            }
        }
        (0..frames)
            .map(|i| {
                let high = |port| {
                    inputs
                        .iter()
                        .filter(|input| input.port == port)
                        .any(|input| {
                            input
                                .block
                                .get(i)
                                .is_some_and(|v| v[0].is_finite() && v[0] > 0.5)
                        })
                };
                let gate = high(1);
                if gate != self.gate_high {
                    self.gate_high = gate;
                    if gate {
                        self.fire();
                    } else {
                        self.release_gate();
                    }
                }
                let retrigger = high(2);
                if retrigger && !self.retrigger_high && self.gated() {
                    self.fire();
                }
                self.retrigger_high = retrigger;
                [self.tick(); 2]
            })
            .collect()
    }
    fn tick(&mut self) -> f32 {
        // Zero-length segments advance before emitting a sample, including A=D=R=0.
        for _ in 0..4 {
            let (duration, target, next) = match self.stage {
                Stage::Idle => {
                    self.level = 0.0;
                    return 0.0;
                }
                Stage::Sustain => {
                    self.level = f64::from(self.sustain);
                    return self.sustain;
                }
                Stage::Attack => (self.attack, 1.0, Stage::Decay),
                Stage::Decay => (
                    self.decay,
                    f64::from(self.sustain),
                    if self.mode == EnvelopeMode::Gate {
                        Stage::Sustain
                    } else {
                        Stage::Release
                    },
                ),
                Stage::Release => (
                    self.release,
                    0.0,
                    if self.mode == EnvelopeMode::Loop && self.gated() {
                        Stage::Attack
                    } else {
                        Stage::Idle
                    },
                ),
            };
            if duration == 0.0 {
                self.level = target;
                self.enter(next);
                continue;
            }
            self.progress = (self.progress
                + 1.0 / (f64::from(duration) * f64::from(self.sample_rate)))
            .min(1.0);
            self.level = self.start_level + (target - self.start_level) * self.progress;
            if self.progress >= 1.0 - 1e-10 {
                self.level = target;
                self.enter(next);
            }
            return self.level as f32;
        }
        self.level as f32
    }
}
impl Source for Adsr {
    fn pull(&mut self, frames: usize) -> Block {
        self.render(frames, &[])
    }
    fn pull_with_inputs(
        &mut self,
        frames: usize,
        _parameters: &[crate::core::graph::ParameterModulation],
        inputs: &[SourceInput],
    ) -> Block {
        self.render(frames, inputs)
    }
    fn start(&mut self) {
        self.drone = true;
        self.fire();
    }
    fn stop(&mut self) {
        self.drone = false;
        self.held.clear();
        self.release_gate();
    }
    fn kill(&mut self) {
        self.drone = false;
        self.held.clear();
        self.level = 0.0;
        self.enter(Stage::Idle);
    }
    fn start_note(&mut self, note: Note, _velocity: f32) {
        self.held.retain(|held| *held != note);
        self.held.push(note);
        self.fire();
    }
    fn stop_note(&mut self, note: Note) {
        let was_held = self.held.contains(&note);
        self.held.retain(|held| *held != note);
        if was_held {
            self.release_gate();
        }
    }
    fn kill_note(&mut self, note: Note) {
        self.held.retain(|held| *held != note);
        if self.held.is_empty() && !self.drone {
            self.kill();
        }
    }
    fn is_active(&self) -> bool {
        self.stage != Stage::Idle
    }
    fn supports_parameter(&self, name: &str) -> bool {
        matches!(name, "attack" | "decay" | "sustain" | "release")
    }
    fn set_parameter(&mut self, name: &str, value: f32) -> bool {
        self.set_parameter_value(name, ParameterValue::Float(f64::from(value)))
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn gate(values: &[f32]) -> SourceInput {
        SourceInput {
            port: 1,
            block: std::sync::Arc::new(values.iter().map(|v| [*v; 2]).collect()),
        }
    }
    #[test]
    fn sample_gates_and_retriggers_keep_edges_across_block_boundaries() {
        let mut e = Adsr::new(48000.0);
        e.attack = 0.0;
        e.decay = 0.0;
        e.release = 0.0;
        e.start(); // A connected low gate takes precedence over the drone start.
        assert_eq!(
            e.pull_with_inputs(2, &[], &[gate(&[0.0, 0.0])]),
            vec![[0.0; 2]; 2]
        );
        assert_eq!(
            e.pull_with_inputs(4, &[], &[gate(&[1.0, 1.0, 0.0, 1.0])]),
            vec![[0.7; 2], [0.7; 2], [0.0; 2], [0.7; 2]]
        );
        e.attack = 0.001;
        e.retrigger = RetriggerMode::Reset;
        let edge = SourceInput {
            port: 2,
            block: std::sync::Arc::new(vec![[1.0; 2]; 2]),
        };
        let output = e.pull_with_inputs(2, &[], &[gate(&[1.0, 1.0]), edge]);
        assert!(output[0][0] < 0.03);
        assert!(output[1][0] > output[0][0]);
    }
    #[test]
    fn one_shot_ignores_note_off_and_loop_repeats_until_the_gate_is_released() {
        let note = Note::from_midi(60);
        let mut e = Adsr::new(48000.0);
        e.attack = 0.001;
        e.decay = 0.0;
        e.release = 0.001;
        e.mode = EnvelopeMode::OneShot;
        e.start_note(note, 1.0);
        e.pull(12);
        e.stop_note(note);
        assert!(e.pull(12).last().unwrap()[0] > 0.49);
        e.pull(100);
        assert!(!e.is_active());
        e.mode = EnvelopeMode::Loop;
        e.start_note(note, 1.0);
        let output = e.pull(240);
        let peaks = output
            .windows(2)
            .filter(|w| w[0][0] > 0.99 && w[1][0] < w[0][0])
            .count();
        assert!(peaks >= 2);
        e.stop_note(note);
        e.pull(100);
        assert!(!e.is_active());
    }
    #[test]
    fn legato_preserves_the_envelope_and_reset_retriggers_from_zero() {
        let mut e = Adsr::new(48000.0);
        e.attack = 0.125;
        e.start_note(Note::from_midi(60), 1.0);
        e.pull(3000);
        e.retrigger = RetriggerMode::Legato;
        e.start_note(Note::from_midi(64), 1.0);
        e.pull(3000);
        assert_eq!(e.level, 1.0);
        e.retrigger = RetriggerMode::Reset;
        e.start_note(Note::from_midi(67), 1.0);
        assert!(e.pull(1)[0][0] < 0.001);
    }
    #[test]
    fn stages_follow_host_sample_rate_and_release_the_instantaneous_level() {
        for rate in [8000.0, 48000.0, 96000.0] {
            let mut e = Adsr::new(rate);
            e.attack = 0.125;
            e.decay = 0.125;
            e.sustain = 0.5;
            e.release = 0.125;
            assert!(e.pull(10).iter().all(|f| *f == [0.0; 2]));
            e.start();
            let half = (rate / 16.0) as usize;
            let attack = e.pull(half);
            assert!((attack.last().unwrap()[0] - 0.5).abs() < 1e-5);
            e.stop();
            let release = e.pull(half);
            assert!((release.last().unwrap()[0] - 0.25).abs() < 1e-5);
            e.pull(half);
            assert!(!e.is_active());
            e.start();
            e.pull(half * 2);
            assert_eq!(e.level, 1.0);
            e.pull(half * 2);
            assert_eq!(e.level, 0.5);
            assert!(e.pull(100).iter().all(|f| *f == [0.5; 2]));
            e.kill();
            assert_eq!(e.pull(1), vec![[0.0; 2]]);
        }
    }
    #[test]
    fn zero_times_overlap_retrigger_and_unknown_note_off_are_defined() {
        let mut e = Adsr::new(48000.0);
        e.attack = 0.0;
        e.decay = 0.0;
        e.release = 0.0;
        let a = Note::from_midi(60);
        let b = Note::from_midi(64);
        e.start_note(a, 1.0);
        assert_eq!(e.pull(1)[0], [0.7; 2]);
        e.start_note(b, 1.0);
        e.stop_note(a);
        assert_eq!(e.pull(1)[0], [0.7; 2]);
        e.stop_note(a);
        assert!(e.is_active());
        e.stop_note(b);
        assert_eq!(e.pull(1)[0], [0.0; 2]);
        assert!(!e.is_active());
        e.attack = 0.125;
        e.start_note(a, 1.0);
        e.pull(1500);
        let before = e.level;
        e.start_note(b, 1.0);
        let next = e.pull(1)[0][0];
        assert!(f64::from(next) >= before && f64::from(next) - before < 0.001);
    }
}
