use rustic_meta::{FilterInfo, MetaGenerator, MetaSink, Parameter};

pub mod traits;

use crate::core::generator::prelude::Waveform;
use crate::core::graph::Filter;

/// Registration entry for a filter type, submitted automatically by `#[derive(FilterMetaData)]`.
pub struct FilterRegistration {
    pub info: fn() -> FilterInfo,
    pub create: fn() -> Box<dyn Filter>,
}

inventory::collect!(FilterRegistration);

pub fn get_filters() -> Vec<FilterInfo> {
    inventory::iter::<FilterRegistration>()
        .map(|r| (r.info)())
        .collect()
}

pub fn get_generators() -> Vec<MetaGenerator> {
    let freq = Parameter::Range {
        title: "Frequency",
        field_name: "frequency",
        min: 1.0,
        max: 20000.0,
        default: 440.0,
        value: 440.0,
    };
    let amp = Parameter::Range {
        title: "Amplitude",
        field_name: "amplitude",
        min: 0.0,
        max: 1.0,
        default: 0.5,
        value: 0.5,
    };
    let attack = Parameter::Range {
        title: "Attack",
        field_name: "attack",
        min: 0.001,
        max: 5.0,
        default: 0.01,
        value: 0.01,
    };
    let decay = Parameter::Range {
        title: "Decay",
        field_name: "decay",
        min: 0.001,
        max: 5.0,
        default: 0.1,
        value: 0.1,
    };
    let sustain = Parameter::Range {
        title: "Sustain",
        field_name: "sustain",
        min: 0.0,
        max: 1.0,
        default: 0.8,
        value: 0.8,
    };
    let release = Parameter::Range {
        title: "Release",
        field_name: "release",
        min: 0.001,
        max: 5.0,
        default: 0.3,
        value: 0.3,
    };

    Waveform::all()
        .into_iter()
        .map(|w| {
            let mut parameters = if w.has_frequency() {
                vec![freq.clone(), amp.clone()]
            } else {
                vec![amp.clone()]
            };
            // Curve params are intentionally hidden in the frontend node UI but must live
            // in the parameter list so the BaklavaJS node interface map carries them and
            // the setValue subscription bridge can forward changes to the backend.
            let attack_curve = Parameter::Range {
                title: "Attack Curve",
                field_name: "attack_curve",
                min: -1.0,
                max: 1.0,
                default: 0.0,
                value: 0.0,
            };
            let decay_curve = Parameter::Range {
                title: "Decay Curve",
                field_name: "decay_curve",
                min: -1.0,
                max: 1.0,
                default: 0.0,
                value: 0.0,
            };
            let release_curve = Parameter::Range {
                title: "Release Curve",
                field_name: "release_curve",
                min: -1.0,
                max: 1.0,
                default: 0.0,
                value: 0.0,
            };
            let attack_cp_t = Parameter::Range {
                title: "Attack CP T",
                field_name: "attack_cp_t",
                min: 0.0,
                max: 1.0,
                default: 0.5,
                value: 0.5,
            };
            let decay_cp_t = Parameter::Range {
                title: "Decay CP T",
                field_name: "decay_cp_t",
                min: 0.0,
                max: 1.0,
                default: 0.5,
                value: 0.5,
            };
            let release_cp_t = Parameter::Range {
                title: "Release CP T",
                field_name: "release_cp_t",
                min: 0.0,
                max: 1.0,
                default: 0.5,
                value: 0.5,
            };
            parameters.extend([
                attack.clone(),
                decay.clone(),
                sustain.clone(),
                release.clone(),
                attack_curve,
                decay_curve,
                release_curve,
                attack_cp_t,
                decay_cp_t,
                release_cp_t,
            ]);
            MetaGenerator {
                name: w.display_name(),
                type_id: w.type_id(),
                description: w.description(),
                parameters,
                output_count: 1,
            }
        })
        .collect()
}

pub fn get_sinks() -> Vec<MetaSink> {
    vec![MetaSink {
        name: "Audio Output",
        description: "Outputs audio to the system audio device",
        input_count: 1,
    }]
}
