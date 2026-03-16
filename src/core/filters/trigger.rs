//! Trigger filter — 8-input bus with gate-driven ADSR envelope.
//!
//! Acts as both a group controller (gate starts/stops all connected upstream
//! generators via the toolkit) and applies a shared ADSR envelope to the
//! combined audio output.
//!
//! `gate` parameter:
//!  1.0  → trigger attack (restart envelope)
//!  0.0  → trigger release
//! -1.0  → kill immediately (output silence)

use std::fmt;

use crate::core::audio::{Block, CHANNELS, silent_block};
use crate::core::envelope::{
    Envelope,
    prelude::{ADSREnvelope, ADSREnvelopeBuilder, BezierSegment, ConstantSegment},
};
use crate::core::graph::{Entry, Filter};
use rustic_meta::{FilterInfo, FilterInput, MetaFilter, Parameter};

/// Copies the same control-point Y helper from `simple_source.rs`.
fn control_y(from: f32, to: f32, curve: f32) -> f32 {
    from + (curve.clamp(-1.0, 1.0) + 1.0) * 0.5 * (to - from)
}

#[derive(Clone, Debug)]
pub struct TriggerFilter {
    inputs: [Block; 8],
    sample_rate: f32,
    // ADSR timing (seconds)
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
    // Curve scalars [-1, 1]
    attack_curve: f32,
    decay_curve: f32,
    release_curve: f32,
    // Bézier control-point X positions [0, 1]
    attack_cp_t: f32,
    decay_cp_t: f32,
    release_cp_t: f32,
    // Built envelope (rebuilt on every param change)
    envelope: ADSREnvelope,
    // Gate state
    gate: f32,     // 1.0=playing, 0.0=releasing, <0=killed
    active: bool,  // true when the envelope is running
    time: f32,     // seconds since last trigger_attack
    note_off: f32, // absolute time of release (0.0 = not yet released)
}

impl Default for TriggerFilter {
    fn default() -> Self {
        let mut f = Self {
            inputs: Default::default(),
            sample_rate: 44100.0,
            attack: 0.01,
            decay: 0.1,
            sustain: 0.8,
            release: 0.3,
            attack_curve: 0.0,
            decay_curve: 0.0,
            release_curve: 0.0,
            attack_cp_t: 0.5,
            decay_cp_t: 0.5,
            release_cp_t: 0.5,
            envelope: ADSREnvelope::default(),
            gate: 0.0,
            active: false,
            time: 0.0,
            note_off: 0.0,
        };
        f.rebuild_envelope();
        f
    }
}

impl TriggerFilter {
    /// Rebuild the ADSR envelope from current parameter fields.
    fn rebuild_envelope(&mut self) {
        let s = self.sustain.clamp(0.0, 1.0);

        let envelope = ADSREnvelopeBuilder::new()
            .attack(Box::new(BezierSegment::new(
                0.0,
                1.0,
                self.attack.max(0.001),
                (self.attack_cp_t, control_y(0.0, 1.0, self.attack_curve)),
            )))
            .decay(Box::new(BezierSegment::new(
                1.0,
                s,
                self.decay.max(0.001),
                (self.decay_cp_t, control_y(1.0, s, self.decay_curve)),
            )))
            .sustain(Box::new(ConstantSegment::new(s, None)))
            .release(Box::new(BezierSegment::new(
                s,
                0.0,
                self.release.max(0.001),
                (self.release_cp_t, control_y(s, 0.0, self.release_curve)),
            )))
            .build();

        self.envelope = envelope;
    }
}

impl fmt::Display for TriggerFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Trigger")
    }
}

impl Entry for TriggerFilter {
    fn push(&mut self, block: Block, port: usize) {
        if port < 8 {
            self.inputs[port] = block;
        }
    }
}

impl Filter for TriggerFilter {
    fn transform(&mut self) -> Vec<Block> {
        let block_size = self
            .inputs
            .iter()
            .map(|b| b.len())
            .find(|&n| n > 0)
            .unwrap_or(512);

        // Always consume inputs so they don't accumulate
        let mut out = silent_block(block_size);
        for inp in self.inputs.iter_mut() {
            for (o, i) in out.iter_mut().zip(inp.iter()) {
                for ch in 0..CHANNELS {
                    o[ch] += i[ch];
                }
            }
            *inp = Vec::new();
        }

        // Kill: discard summed audio
        if self.gate < 0.0 || !self.active {
            return vec![silent_block(block_size)];
        }

        let dt = 1.0 / self.sample_rate.max(1.0);
        for frame in out.iter_mut() {
            let gain = self.envelope.at(self.time, self.note_off);
            for ch in frame.iter_mut() {
                *ch *= gain;
            }
            self.time += dt;
            // Check if release phase has fully completed
            if self.note_off > 0.0 && self.envelope.completed(self.time, self.note_off) {
                self.active = false;
                break;
            }
        }

        vec![out]
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl MetaFilter for TriggerFilter {
    fn set_parameter(&mut self, name: &str, value: f32) {
        match name {
            "attack" => {
                self.attack = value.max(0.001);
                self.rebuild_envelope();
            }
            "decay" => {
                self.decay = value.max(0.001);
                self.rebuild_envelope();
            }
            "sustain" => {
                self.sustain = value.clamp(0.0, 1.0);
                self.rebuild_envelope();
            }
            "release" => {
                self.release = value.max(0.001);
                self.rebuild_envelope();
            }
            "attack_curve" => {
                self.attack_curve = value.clamp(-1.0, 1.0);
                self.rebuild_envelope();
            }
            "decay_curve" => {
                self.decay_curve = value.clamp(-1.0, 1.0);
                self.rebuild_envelope();
            }
            "release_curve" => {
                self.release_curve = value.clamp(-1.0, 1.0);
                self.rebuild_envelope();
            }
            "attack_cp_t" => {
                self.attack_cp_t = value.clamp(0.0, 1.0);
                self.rebuild_envelope();
            }
            "decay_cp_t" => {
                self.decay_cp_t = value.clamp(0.0, 1.0);
                self.rebuild_envelope();
            }
            "release_cp_t" => {
                self.release_cp_t = value.clamp(0.0, 1.0);
                self.rebuild_envelope();
            }
            "sample_rate" => {
                self.sample_rate = value.max(1.0);
            }
            "gate" => {
                self.gate = value;
                if value >= 1.0 {
                    // (Re)start attack
                    self.active = true;
                    self.time = 0.0;
                    self.note_off = 0.0;
                } else if value == 0.0 && self.active && self.note_off == 0.0 {
                    // Trigger release from current position
                    self.note_off = self.time.max(f32::EPSILON);
                }
                // value < 0.0: kill handled in transform()
            }
            _ => {}
        }
    }

    fn metadata() -> FilterInfo {
        FilterInfo {
            name: "Trigger",
            description: "8-input summing bus with gate-driven ADSR envelope",
            inputs: vec![
                FilterInput {
                    label: Some("Input 1"),
                    parameter: None,
                },
                FilterInput {
                    label: Some("Input 2"),
                    parameter: None,
                },
                FilterInput {
                    label: Some("Input 3"),
                    parameter: None,
                },
                FilterInput {
                    label: Some("Input 4"),
                    parameter: None,
                },
                FilterInput {
                    label: Some("Input 5"),
                    parameter: None,
                },
                FilterInput {
                    label: Some("Input 6"),
                    parameter: None,
                },
                FilterInput {
                    label: Some("Input 7"),
                    parameter: None,
                },
                FilterInput {
                    label: Some("Input 8"),
                    parameter: None,
                },
                FilterInput {
                    label: None,
                    parameter: Some(Parameter::Range {
                        title: "Attack",
                        field_name: "attack",
                        min: 0.001,
                        max: 5.0,
                        default: 0.01,
                        value: 0.01,
                    }),
                },
                FilterInput {
                    label: None,
                    parameter: Some(Parameter::Range {
                        title: "Decay",
                        field_name: "decay",
                        min: 0.001,
                        max: 5.0,
                        default: 0.1,
                        value: 0.1,
                    }),
                },
                FilterInput {
                    label: None,
                    parameter: Some(Parameter::Range {
                        title: "Sustain",
                        field_name: "sustain",
                        min: 0.0,
                        max: 1.0,
                        default: 0.8,
                        value: 0.8,
                    }),
                },
                FilterInput {
                    label: None,
                    parameter: Some(Parameter::Range {
                        title: "Release",
                        field_name: "release",
                        min: 0.001,
                        max: 5.0,
                        default: 0.3,
                        value: 0.3,
                    }),
                },
                FilterInput {
                    label: None,
                    parameter: Some(Parameter::Range {
                        title: "Attack Curve",
                        field_name: "attack_curve",
                        min: -1.0,
                        max: 1.0,
                        default: 0.0,
                        value: 0.0,
                    }),
                },
                FilterInput {
                    label: None,
                    parameter: Some(Parameter::Range {
                        title: "Decay Curve",
                        field_name: "decay_curve",
                        min: -1.0,
                        max: 1.0,
                        default: 0.0,
                        value: 0.0,
                    }),
                },
                FilterInput {
                    label: None,
                    parameter: Some(Parameter::Range {
                        title: "Release Curve",
                        field_name: "release_curve",
                        min: -1.0,
                        max: 1.0,
                        default: 0.0,
                        value: 0.0,
                    }),
                },
                FilterInput {
                    label: None,
                    parameter: Some(Parameter::Range {
                        title: "Attack CP T",
                        field_name: "attack_cp_t",
                        min: 0.0,
                        max: 1.0,
                        default: 0.5,
                        value: 0.5,
                    }),
                },
                FilterInput {
                    label: None,
                    parameter: Some(Parameter::Range {
                        title: "Decay CP T",
                        field_name: "decay_cp_t",
                        min: 0.0,
                        max: 1.0,
                        default: 0.5,
                        value: 0.5,
                    }),
                },
                FilterInput {
                    label: None,
                    parameter: Some(Parameter::Range {
                        title: "Release CP T",
                        field_name: "release_cp_t",
                        min: 0.0,
                        max: 1.0,
                        default: 0.5,
                        value: 0.5,
                    }),
                },
            ],
            outputs: 1,
        }
    }
}

fn create_trigger() -> Box<dyn Filter> {
    Box::new(TriggerFilter::default())
}

fn trigger_filter_info() -> FilterInfo {
    TriggerFilter::metadata()
}

inventory::submit!(crate::meta::FilterRegistration {
    info: trigger_filter_info,
    create: create_trigger,
});
