//! Core instrument registry: name → [`InstrumentSpec`].
//!
//! Built-ins are embedded specs, translated 1:1 from the legacy instrument
//! structs (`Kick::new()`, `SynthConfig::*`, …). Front-ends resolve names
//! here instead of keeping their own hardcoded matches.

use std::collections::HashMap;

use crate::core::generator::prelude::{FrequencyRelation, MixMode, Waveform};
use crate::core::graph::{MonophonicAllocationStrategy, PolyphonicAllocationStrategy};
use crate::instruments::spec::{
    EnvelopeSpec, FxSpec, InstrumentSpec, NoteLifecycle, SegmentSpec, SpecError, ToneSpec,
    VoiceSpec, validate_spec,
};

/// Registry of instrument specs, keyed by `spec.name`.
#[derive(Debug, Clone)]
pub struct InstrumentRegistry {
    specs: HashMap<String, InstrumentSpec>,
}

impl Default for InstrumentRegistry {
    fn default() -> Self {
        Self::built_in()
    }
}

impl InstrumentRegistry {
    /// An empty registry (no built-ins).
    pub fn empty() -> Self {
        Self {
            specs: HashMap::new(),
        }
    }

    /// A registry pre-loaded with every built-in instrument.
    pub fn built_in() -> Self {
        let mut registry = Self::empty();
        for spec in built_in_specs() {
            // Built-ins are validated by unit test; user specs go through register()
            registry.specs.insert(spec.name.clone(), spec);
        }
        registry
    }

    pub fn get(&self, name: &str) -> Option<&InstrumentSpec> {
        self.specs.get(name)
    }

    /// Validate and insert a spec (definition-time validation — unknown
    /// filters or params are rejected here, not at trigger time).
    /// Replaces any existing spec with the same name.
    pub fn register(&mut self, spec: InstrumentSpec) -> Result<(), SpecError> {
        validate_spec(&spec)?;
        self.specs.insert(spec.name.clone(), spec);
        Ok(())
    }

    /// Remove a spec by name. Returns whether it existed.
    pub fn unregister(&mut self, name: &str) -> bool {
        self.specs.remove(name).is_some()
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.specs.keys().map(String::as_str)
    }

    pub fn specs(&self) -> impl Iterator<Item = &InstrumentSpec> {
        self.specs.values()
    }

    pub fn len(&self) -> usize {
        self.specs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.specs.is_empty()
    }

    /// Percussive = pitch-agnostic mono voice; derived from the spec rather
    /// than a separate list (replaces the TUI's `PERCUSSION` const).
    pub fn is_percussion(&self, name: &str) -> bool {
        matches!(
            self.specs.get(name).map(|s| &s.voice),
            Some(VoiceSpec::Mono {
                track_pitch: false,
                ..
            })
        )
    }
}

// ---------------------------------------------------------------------------
// Built-in specs
// ---------------------------------------------------------------------------

fn built_in_specs() -> Vec<InstrumentSpec> {
    let mut specs = vec![kick(), snare(), hihat(), clap(), rim(), tom()];
    specs.extend(SYNTH_PRESETS.iter().map(
        |&(name, ref waveform, voices, adsr, lowpass_cutoff, gain)| {
            synth(name, waveform.clone(), voices, adsr, lowpass_cutoff, gain)
        },
    ));
    specs
}

fn percussive_voice() -> VoiceSpec {
    VoiceSpec::Mono {
        track_pitch: false,
        allocation: MonophonicAllocationStrategy::Replace,
    }
}

/// Mirrors `Kick::new()` (`drum/kick.rs`).
fn kick() -> InstrumentSpec {
    InstrumentSpec {
        name: "kick".into(),
        note_lifecycle: NoteLifecycle::OneShot,
        voice: percussive_voice(),
        tones: vec![
            ToneSpec {
                waveform: Waveform::WhiteNoise,
                frequency_relation: Some(FrequencyRelation::Constant(1.0)),
                frequency: None,
                amplitude_envelope: Some(EnvelopeSpec::Segments {
                    attack: SegmentSpec::Bezier {
                        from: 0.0,
                        to: 0.1,
                        duration: 0.001,
                        control: (0.0, 1.0),
                    },
                    decay: SegmentSpec::Linear {
                        from: 0.1,
                        to: 0.0,
                        duration: 0.1,
                    },
                    sustain: None,
                    release: SegmentSpec::Constant {
                        value: 0.0,
                        duration: Some(0.0),
                    },
                }),
            },
            ToneSpec {
                waveform: Waveform::Sine,
                frequency_relation: Some(FrequencyRelation::Ratio(1.0)),
                frequency: None,
                amplitude_envelope: Some(EnvelopeSpec::Segment(SegmentSpec::Constant {
                    value: 1.0,
                    duration: None,
                })),
            },
        ],
        mix_mode: MixMode::Sum,
        pitch_envelope: Some(EnvelopeSpec::Segment(SegmentSpec::Bezier {
            from: 1.0,
            to: 0.5,
            duration: 0.3,
            control: (2.0, 0.2),
        })),
        amplitude_envelope: Some(EnvelopeSpec::Segments {
            attack: SegmentSpec::Bezier {
                from: 0.0,
                to: 1.0,
                duration: 0.001,
                control: (0.0, 1.0),
            },
            decay: SegmentSpec::Linear {
                from: 1.0,
                to: 0.0,
                duration: 0.3,
            },
            sustain: None,
            release: SegmentSpec::Linear {
                from: 0.0,
                to: 0.0,
                duration: 0.0,
            },
        }),
        base_frequency: Some(58.0),
        fx: vec![],
        gain: 1.0,
        velocity_sensitivity: 1.0,
        mods: vec![],
    }
}

/// Mirrors `Snare::new()` (`drum/snare.rs`).
fn snare() -> InstrumentSpec {
    InstrumentSpec {
        name: "snare".into(),
        note_lifecycle: NoteLifecycle::OneShot,
        voice: percussive_voice(),
        tones: vec![
            ToneSpec {
                waveform: Waveform::WhiteNoise,
                frequency_relation: Some(FrequencyRelation::Constant(1.0)),
                frequency: None,
                amplitude_envelope: Some(EnvelopeSpec::Segment(SegmentSpec::Constant {
                    value: 0.1,
                    duration: None,
                })),
            },
            ToneSpec {
                waveform: Waveform::Sine,
                frequency_relation: Some(FrequencyRelation::Ratio(1.0)),
                frequency: None,
                amplitude_envelope: None,
            },
        ],
        mix_mode: MixMode::Sum,
        pitch_envelope: Some(EnvelopeSpec::Segment(SegmentSpec::Bezier {
            from: 1.2,
            to: 0.8,
            duration: 0.2,
            control: (0.0, 1.0),
        })),
        amplitude_envelope: Some(EnvelopeSpec::Segments {
            attack: SegmentSpec::Bezier {
                from: 0.0,
                to: 1.0,
                duration: 0.05,
                control: (0.0, 1.0),
            },
            decay: SegmentSpec::Linear {
                from: 1.0,
                to: 0.0,
                duration: 0.1,
            },
            sustain: None,
            release: SegmentSpec::Linear {
                from: 0.0,
                to: 0.0,
                duration: 0.0,
            },
        }),
        base_frequency: Some(155.0),
        fx: vec![],
        gain: 1.0,
        velocity_sensitivity: 1.0,
        mods: vec![],
    }
}

/// Natural closed hi-hat: a broadband noise burst carries the body while a few
/// quiet, inharmonic sine partials add cymbal shimmer without the harsh upper
/// harmonics produced by the previous square-wave bank.
fn hihat() -> InstrumentSpec {
    let tone = |waveform, frequency, level| ToneSpec {
        waveform,
        frequency_relation: None,
        frequency,
        amplitude_envelope: Some(EnvelopeSpec::Segment(SegmentSpec::Constant {
            value: level,
            duration: None,
        })),
    };

    InstrumentSpec {
        name: "hihat".into(),
        note_lifecycle: NoteLifecycle::OneShot,
        voice: percussive_voice(),
        tones: vec![
            tone(Waveform::WhiteNoise, None, 0.82),
            tone(Waveform::Sine, Some(6_713.0), 0.14),
            tone(Waveform::Sine, Some(8_923.0), 0.09),
            tone(Waveform::Sine, Some(11_317.0), 0.05),
        ],
        mix_mode: MixMode::Sum,
        pitch_envelope: None,
        amplitude_envelope: Some(EnvelopeSpec::Segments {
            attack: SegmentSpec::Bezier {
                from: 0.0,
                to: 1.0,
                duration: 0.0008,
                control: (0.0, 1.0),
            },
            decay: SegmentSpec::Bezier {
                from: 1.0,
                to: 0.0,
                duration: 0.09,
                control: (0.16, 0.015),
            },
            sustain: Some(SegmentSpec::Constant {
                value: 0.0,
                duration: None,
            }),
            release: SegmentSpec::Constant {
                value: 0.0,
                duration: None,
            },
        }),
        base_frequency: None,
        fx: vec![
            FxSpec {
                type_id: "HighPassFilter".into(),
                params: HashMap::from([("cutoff_frequency".into(), 4800.0)]),
            },
            FxSpec {
                type_id: "ResonantBandpassFilter".into(),
                params: HashMap::from([
                    ("center_frequency".into(), 9200.0),
                    ("quality".into(), 0.55),
                ]),
            },
        ],
        gain: 0.72,
        velocity_sensitivity: 1.0,
        mods: vec![],
    }
}

/// Mirrors `Clap::new()` (`drum/percussion.rs`).
fn clap() -> InstrumentSpec {
    InstrumentSpec {
        name: "clap".into(),
        note_lifecycle: NoteLifecycle::OneShot,
        voice: percussive_voice(),
        tones: vec![ToneSpec {
            waveform: Waveform::WhiteNoise,
            frequency_relation: Some(FrequencyRelation::Constant(1.0)),
            frequency: None,
            amplitude_envelope: Some(EnvelopeSpec::Segments {
                attack: SegmentSpec::Bezier {
                    from: 0.0,
                    to: 1.0,
                    duration: 0.001,
                    control: (0.0, 1.0),
                },
                decay: SegmentSpec::Linear {
                    from: 1.0,
                    to: 0.0,
                    duration: 0.06,
                },
                sustain: None,
                release: SegmentSpec::Constant {
                    value: 0.0,
                    duration: Some(0.0),
                },
            }),
        }],
        mix_mode: MixMode::Sum,
        pitch_envelope: None,
        amplitude_envelope: None,
        base_frequency: None,
        fx: vec![],
        gain: 0.9,
        velocity_sensitivity: 1.0,
        mods: vec![],
    }
}

/// Mirrors `Rim::new()` (`drum/percussion.rs`).
fn rim() -> InstrumentSpec {
    InstrumentSpec {
        name: "rim".into(),
        note_lifecycle: NoteLifecycle::OneShot,
        voice: percussive_voice(),
        tones: vec![ToneSpec {
            waveform: Waveform::Sine,
            frequency_relation: Some(FrequencyRelation::Ratio(1.0)),
            frequency: None,
            amplitude_envelope: None,
        }],
        mix_mode: MixMode::Sum,
        pitch_envelope: None,
        amplitude_envelope: Some(EnvelopeSpec::Segments {
            attack: SegmentSpec::Bezier {
                from: 0.0,
                to: 1.0,
                duration: 0.001,
                control: (0.0, 1.0),
            },
            decay: SegmentSpec::Linear {
                from: 1.0,
                to: 0.0,
                duration: 0.04,
            },
            sustain: None,
            release: SegmentSpec::Constant {
                value: 0.0,
                duration: Some(0.0),
            },
        }),
        base_frequency: Some(800.0),
        fx: vec![],
        gain: 0.85,
        velocity_sensitivity: 1.0,
        mods: vec![],
    }
}

/// Mirrors `Tom::new()` (`drum/percussion.rs`).
fn tom() -> InstrumentSpec {
    InstrumentSpec {
        name: "tom".into(),
        note_lifecycle: NoteLifecycle::OneShot,
        voice: percussive_voice(),
        tones: vec![ToneSpec {
            waveform: Waveform::Sine,
            frequency_relation: Some(FrequencyRelation::Ratio(1.0)),
            frequency: None,
            amplitude_envelope: None,
        }],
        mix_mode: MixMode::Sum,
        pitch_envelope: Some(EnvelopeSpec::Segment(SegmentSpec::Bezier {
            from: 1.5,
            to: 0.6,
            duration: 0.15,
            control: (0.0, 1.0),
        })),
        amplitude_envelope: Some(EnvelopeSpec::Segments {
            attack: SegmentSpec::Bezier {
                from: 0.0,
                to: 1.0,
                duration: 0.002,
                control: (0.0, 1.0),
            },
            decay: SegmentSpec::Linear {
                from: 1.0,
                to: 0.0,
                duration: 0.25,
            },
            sustain: None,
            release: SegmentSpec::Constant {
                value: 0.0,
                duration: Some(0.0),
            },
        }),
        base_frequency: Some(120.0),
        fx: vec![],
        gain: 0.95,
        velocity_sensitivity: 1.0,
        mods: vec![],
    }
}

/// One row per `SynthConfig` preset (`synth/mod.rs`):
/// (name, waveform, voices, (attack, decay, sustain, release), lowpass cutoff, gain)
type SynthPreset = (
    &'static str,
    Waveform,
    usize,
    (f32, f32, f32, f32),
    Option<f32>,
    f32,
);

const SYNTH_PRESETS: [SynthPreset; 9] = [
    ("sine", Waveform::Sine, 8, (0.01, 0.1, 0.8, 0.2), None, 0.8),
    (
        "saw",
        Waveform::Sawtooth,
        8,
        (0.02, 0.15, 0.6, 0.15),
        Some(4000.0),
        0.7,
    ),
    (
        "square",
        Waveform::Square,
        8,
        (0.01, 0.1, 0.5, 0.1),
        Some(3000.0),
        0.6,
    ),
    (
        "triangle",
        Waveform::Triangle,
        8,
        (0.02, 0.2, 0.7, 0.25),
        None,
        0.75,
    ),
    (
        "piano",
        Waveform::Sine,
        8,
        (0.005, 0.25, 0.4, 0.35),
        Some(6500.0),
        0.55,
    ),
    (
        "bass",
        Waveform::Square,
        4,
        (0.005, 0.08, 0.7, 0.1),
        Some(800.0),
        0.9,
    ),
    (
        "pad",
        Waveform::Sawtooth,
        6,
        (0.4, 0.6, 0.75, 1.2),
        Some(1200.0),
        0.55,
    ),
    (
        "pluck",
        Waveform::Triangle,
        6,
        (0.001, 0.08, 0.05, 0.15),
        Some(5000.0),
        0.8,
    ),
    (
        "bell",
        Waveform::Sine,
        4,
        (0.001, 0.6, 0.0, 0.8),
        Some(8000.0),
        0.7,
    ),
];

/// Mirrors `Synth::new(SynthConfig)` (`synth/mod.rs`).
fn synth(
    name: &str,
    waveform: Waveform,
    voices: usize,
    (attack, decay, sustain, release): (f32, f32, f32, f32),
    lowpass_cutoff: Option<f32>,
    gain: f32,
) -> InstrumentSpec {
    let tones = if name == "piano" {
        let partial = |relation, level| ToneSpec {
            waveform: Waveform::Sine,
            frequency_relation: Some(relation),
            frequency: None,
            amplitude_envelope: Some(EnvelopeSpec::Segment(SegmentSpec::Constant {
                value: level,
                duration: None,
            })),
        };
        vec![
            partial(FrequencyRelation::Identity, 1.0),
            partial(FrequencyRelation::Harmonic(2), 0.42),
            partial(FrequencyRelation::Harmonic(3), 0.2),
            partial(FrequencyRelation::Harmonic(4), 0.09),
        ]
    } else {
        vec![ToneSpec {
            waveform,
            frequency_relation: Some(FrequencyRelation::Identity),
            frequency: None,
            amplitude_envelope: None,
        }]
    };
    InstrumentSpec {
        name: name.into(),
        note_lifecycle: NoteLifecycle::Gated,
        voice: VoiceSpec::Poly {
            voices,
            allocation: PolyphonicAllocationStrategy::default(),
        },
        tones,
        mix_mode: MixMode::Sum,
        pitch_envelope: None,
        amplitude_envelope: Some(EnvelopeSpec::Adsr {
            attack,
            decay,
            sustain,
            release,
        }),
        base_frequency: None,
        fx: lowpass_cutoff
            .map(|cutoff| FxSpec {
                type_id: "LowPassFilter".into(),
                params: HashMap::from([("cutoff_frequency".into(), cutoff)]),
            })
            .into_iter()
            .collect(),
        gain,
        velocity_sensitivity: 1.0,
        mods: vec![],
    }
}
