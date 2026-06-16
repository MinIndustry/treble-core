//! Spec module
//! This module defines the specification for custom instruments.
//! The aim of this module is to slowly replace the hardcoded instruments

use std::collections::HashMap;

use petgraph::graph::NodeIndex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::core::envelope::Envelope;
use crate::core::envelope::prelude::{ADSREnvelope, ADSREnvelopeBuilder, LinearSegment};
use crate::core::generator::prelude::MixMode;
use crate::core::generator::prelude::SingleToneGenerator;
use crate::core::generator::prelude::builder::MultiToneGeneratorBuilder;
use crate::core::graph::{
    Filter, MonophonicAllocationStrategy, MonophonicSource, PolyphonicSource, SimpleSink, Source,
    System,
};
use crate::core::{
    generator::prelude::{FrequencyRelation, Waveform},
    graph::PolyphonicAllocationStrategy,
};

#[derive(Error, Debug)]
pub enum SpecError {
    #[error("Other error: {0}")]
    Other(String),
    #[error("Unknown filter: {0}")]
    UnknownFilter(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum VoiceSpec {
    Mono {
        track_pitch: bool,
        allocation: MonophonicAllocationStrategy,
    },
    Poly {
        voices: usize,
        allocation: PolyphonicAllocationStrategy,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToneSpec {
    pub waveform: Waveform,
    pub frequency_relation: FrequencyRelation,
    pub amplitude_envelope: Option<EnvelopeSpec>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
/// The envelope specification. As of v1, only linear ADSR segments
pub enum EnvelopeSpec {
    Adsr {
        attack: f32,
        decay: f32,
        sustain: f32,
        release: f32,
    },
}

impl EnvelopeSpec {
    pub fn as_dyn_envelope(&self) -> Box<dyn Envelope> {
        match self {
            EnvelopeSpec::Adsr {
                attack,
                decay,
                sustain,
                release,
            } => Box::new(
                ADSREnvelopeBuilder::new()
                    .attack(Box::new(LinearSegment::new(0.0, 1.0, *attack)))
                    .decay(Box::new(LinearSegment::new(1.0, *sustain, *decay)))
                    .sustain(Box::new(LinearSegment::new(*sustain, *sustain, 0.0)))
                    .release(Box::new(LinearSegment::new(*sustain, 0.0, *release)))
                    .build(),
            ),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FxSpec {
    type_id: String,
    params: HashMap<String, f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
/// WIP
pub struct ModSpec;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstrumentSpec {
    pub name: String,
    pub voice: VoiceSpec,
    pub tones: Vec<ToneSpec>,
    pub mix_mode: MixMode,
    pub pitch_envelope: Option<EnvelopeSpec>,
    pub amplitude_envelope: Option<EnvelopeSpec>,
    pub base_frequency: Option<f32>,
    pub fx: Vec<FxSpec>,
    pub gain: f32,
    pub velocity_sensitivity: f32,
    pub mods: Vec<ModSpec>,
}

fn create_filter(node_type: &str, sample_rate: f32) -> Result<Box<dyn Filter>, String> {
    for entry in inventory::iter::<crate::meta::FilterRegistration>() {
        let info = (entry.info)();
        if info.type_id == node_type {
            let mut filter = (entry.create)();
            // Apply sample_rate to any filter that exposes it as a parameter.
            // Filters without a "sample_rate" field will silently ignore this (logged at debug).
            filter.set_parameter("sample_rate", sample_rate);
            return Ok(filter);
        }
    }
    Err(format!("Unknown filter type: {}", node_type))
}

pub fn compile_spec(spec: &InstrumentSpec, sample_rate: f32) -> Result<System, SpecError> {
    // Verify all filters in the fx spec are actually real
    for fx in spec.fx.iter() {
        let mut _testing_filter = (inventory::iter::<crate::meta::FilterRegistration>()
            .find(|filt| (filt.info)().name == fx.type_id.as_str())
            .ok_or(SpecError::UnknownFilter(fx.type_id.clone()))?
            .create)();

        for (_name, _value) in fx.params.iter() {
            // TODO: Implement this test once set_parameter validates the
            // parameter as changed
            // if testing_filter.set_parameter(&name, *value) {
            // }
        }
    }

    let mut generator = MultiToneGeneratorBuilder::new();
    for tone in &spec.tones {
        let amplitude = (tone.amplitude_envelope)
            .as_ref()
            .map(|es| es.as_dyn_envelope())
            .unwrap_or(Box::new(ADSREnvelope::default()));

        generator = generator.add_generator(SingleToneGenerator::new(
            tone.waveform.clone(),
            Some(tone.frequency_relation.clone()),
            None,
            amplitude,
            0.0,
        ));
    }
    let generator = generator
        .mix_mode(spec.mix_mode.clone())
        .frequency(spec.base_frequency.unwrap_or(0.0))
        .amplitude_envelope(Some(
            spec.amplitude_envelope
                .as_ref()
                .map(|es| es.as_dyn_envelope())
                .unwrap_or(Box::new(ADSREnvelope::default())),
        ))
        .pitch_envelope(Some(
            spec.pitch_envelope
                .as_ref()
                .map(|es| es.as_dyn_envelope())
                .unwrap_or(Box::new(ADSREnvelope::default())),
        ))
        .build();

    let system_source: Box<dyn Source> = match &spec.voice {
        VoiceSpec::Mono {
            track_pitch,
            allocation,
        } => {
            if *track_pitch {
                Box::new(MonophonicSource::new(
                    generator,
                    sample_rate,
                    allocation.clone(),
                ))
            } else {
                Box::new(MonophonicSource::new_percussive(
                    generator,
                    sample_rate,
                    allocation.clone(),
                ))
            }
        }
        VoiceSpec::Poly { voices, allocation } => Box::new(PolyphonicSource::new(
            generator,
            *voices,
            sample_rate,
            allocation.clone(),
        )),
    };

    let mut compiled_system = System::new();
    let source_index = compiled_system.add_source(system_source);
    let sink_index = compiled_system.add_sink(Box::new(SimpleSink::new()));

    let number_of_fx = spec.fx.len();
    let mut last_filter_index: Option<NodeIndex<u32>> = None;
    for (idx, fx) in spec.fx.iter().rev().enumerate() {
        // TODO: Maybe merge check and build ?
        let mut filt = create_filter(&fx.type_id, sample_rate).unwrap();
        for (param, value) in fx.params.iter() {
            filt.set_parameter(param, *value);
        }

        let filter_index = compiled_system.add_filter(filt);
        // Last filter (first in the chain, connect to the source)
        if idx + 1 == number_of_fx {
            compiled_system.connect_source(source_index, filter_index, 0);
            continue;
        }

        match last_filter_index {
            // First filter (last in the chain), connect to sink
            None => {
                compiled_system.connect_sink(filter_index, sink_index, 0);
            }
            Some(next_filter_index) => {
                compiled_system.connect(filter_index, next_filter_index, 0, 0);
            }
        }
        last_filter_index = Some(filter_index);
    }

    // If there is no fx, connect the source to the sink
    if spec.fx.is_empty() {
        compiled_system.connect_source_to_sink(source_index, sink_index);
    }

    Ok(compiled_system)
}
