//! Spec module
//! This module defines the specification for custom instruments.
//! The aim of this module is to slowly replace the hardcoded instruments

use std::collections::{HashMap, HashSet};

use petgraph::graph::NodeIndex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::Note;
use crate::core::envelope::Envelope;
use crate::core::envelope::prelude::{
    ADSREnvelopeBuilder, BezierSegment, ConstantSegment, LinearSegment, Segment,
};
use crate::core::filters::prelude::GainFilter;
use crate::core::generator::prelude::MixMode;
use crate::core::generator::prelude::builder::{MultiToneGeneratorBuilder, ToneGeneratorBuilder};
use crate::core::graph::{
    Filter, MonophonicAllocationStrategy, MonophonicSource, PolyphonicSource, SimpleSink, Source,
    System,
};
use crate::core::{
    generator::prelude::{FrequencyRelation, Waveform},
    graph::PolyphonicAllocationStrategy,
};
use crate::instruments::Instrument;
use treble_meta::Parameter;

#[derive(Error, Debug)]
pub enum SpecError {
    #[error("Other error: {0}")]
    Other(String),
    #[error("Unknown filter: {0}")]
    UnknownFilter(String),
    #[error("Unknown parameter '{param}' for filter '{filter}'")]
    UnknownParameter { filter: String, param: String },
    #[error("Graph compute failed: {0}")]
    Compute(String),
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

/// A single envelope segment as serializable data.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SegmentSpec {
    Linear {
        from: f32,
        to: f32,
        duration: f32,
    },
    Bezier {
        from: f32,
        to: f32,
        duration: f32,
        control: (f32, f32),
    },
    Constant {
        value: f32,
        duration: Option<f32>,
    },
}

impl SegmentSpec {
    pub fn as_dyn_segment(&self) -> Box<dyn Segment> {
        match self {
            SegmentSpec::Linear { from, to, duration } => {
                Box::new(LinearSegment::new(*from, *to, *duration))
            }
            SegmentSpec::Bezier {
                from,
                to,
                duration,
                control,
            } => Box::new(BezierSegment::new(*from, *to, *duration, *control)),
            SegmentSpec::Constant { value, duration } => {
                Box::new(ConstantSegment::new(*value, *duration))
            }
        }
    }

    fn as_dyn_envelope(&self) -> Box<dyn Envelope> {
        match self {
            SegmentSpec::Linear { from, to, duration } => {
                Box::new(LinearSegment::new(*from, *to, *duration))
            }
            SegmentSpec::Bezier {
                from,
                to,
                duration,
                control,
            } => Box::new(BezierSegment::new(*from, *to, *duration, *control)),
            SegmentSpec::Constant { value, duration } => {
                Box::new(ConstantSegment::new(*value, *duration))
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToneSpec {
    pub waveform: Waveform,
    /// How this tone's frequency follows the instrument's base frequency.
    /// `None` — the tone keeps its fixed `frequency` (e.g. hihat partials).
    pub frequency_relation: Option<FrequencyRelation>,
    /// Fixed tone frequency in Hz. `None` — the builder default.
    pub frequency: Option<f32>,
    pub amplitude_envelope: Option<EnvelopeSpec>,
}

/// The envelope specification.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum EnvelopeSpec {
    /// Linear ADSR peaking at 1.0 — the common case, kept terse for the DSL.
    Adsr {
        attack: f32,
        decay: f32,
        sustain: f32,
        release: f32,
    },
    /// Full-fidelity ADSR from explicit segments.
    /// `sustain: None` — the builder default (constant at the decay's end value).
    Segments {
        attack: SegmentSpec,
        decay: SegmentSpec,
        sustain: Option<SegmentSpec>,
        release: SegmentSpec,
    },
    /// A single segment used directly as the whole envelope
    /// (pitch sweeps, constant levels).
    Segment(SegmentSpec),
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
                    .sustain(Box::new(ConstantSegment::new(*sustain, None)))
                    .release(Box::new(LinearSegment::new(*sustain, 0.0, *release)))
                    .build(),
            ),
            EnvelopeSpec::Segments {
                attack,
                decay,
                sustain,
                release,
            } => {
                let mut builder = ADSREnvelopeBuilder::new()
                    .attack(attack.as_dyn_segment())
                    .decay(decay.as_dyn_segment())
                    .release(release.as_dyn_segment());
                if let Some(sustain) = sustain {
                    builder = builder.sustain(sustain.as_dyn_segment());
                }
                Box::new(builder.build())
            }
            EnvelopeSpec::Segment(segment) => segment.as_dyn_envelope(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FxSpec {
    pub type_id: String,
    pub params: HashMap<String, f32>,
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

fn param_field_name<'a>(param: &'a Parameter<&'static str>) -> &'a str {
    match param {
        Parameter::Toggle { field_name, .. }
        | Parameter::Range { field_name, .. }
        | Parameter::Float { field_name, .. }
        | Parameter::Int { field_name, .. }
        | Parameter::List { field_name, .. } => field_name,
    }
}

/// Validate a spec against the filter inventory without building anything.
///
/// This is the definition-time check: `type_id`s must exist in the registry
/// and every fx param must be a declared `#[filter_parameter]` of that filter.
pub fn validate_spec(spec: &InstrumentSpec) -> Result<(), SpecError> {
    for fx in spec.fx.iter() {
        let info = inventory::iter::<crate::meta::FilterRegistration>()
            .map(|entry| (entry.info)())
            .find(|info| info.type_id == fx.type_id)
            .ok_or_else(|| SpecError::UnknownFilter(fx.type_id.clone()))?;

        let known: HashSet<&str> = info
            .inputs
            .iter()
            .filter_map(|input| input.parameter.as_ref())
            .map(param_field_name)
            .collect();

        for param in fx.params.keys() {
            if !known.contains(param.as_str()) {
                return Err(SpecError::UnknownParameter {
                    filter: fx.type_id.clone(),
                    param: param.clone(),
                });
            }
        }
    }
    Ok(())
}

fn create_filter(node_type: &str, sample_rate: f32) -> Result<Box<dyn Filter>, SpecError> {
    for entry in inventory::iter::<crate::meta::FilterRegistration>() {
        let info = (entry.info)();
        if info.type_id == node_type {
            let mut filter = (entry.create)();
            // Filters that expose sample_rate as a parameter pick up the
            // engine rate here; others ignore it (logged at debug).
            filter.set_parameter("sample_rate", sample_rate);
            return Ok(filter);
        }
    }
    Err(SpecError::UnknownFilter(node_type.to_string()))
}

pub fn compile_spec(spec: &InstrumentSpec, sample_rate: f32) -> Result<System, SpecError> {
    validate_spec(spec)?;

    let mut generator = MultiToneGeneratorBuilder::new();
    for tone in &spec.tones {
        let mut tone_builder = ToneGeneratorBuilder::new().waveform(tone.waveform.clone());
        if let Some(relation) = &tone.frequency_relation {
            tone_builder = tone_builder.frequency_relation(relation.clone());
        }
        if let Some(frequency) = tone.frequency {
            tone_builder = tone_builder.frequency(frequency);
        }
        if let Some(envelope) = &tone.amplitude_envelope {
            tone_builder = tone_builder.amplitude_envelope(envelope.as_dyn_envelope());
        }
        generator = generator.add_generator(tone_builder.build());
    }

    generator = generator
        .mix_mode(spec.mix_mode.clone())
        .amplitude_envelope(
            spec.amplitude_envelope
                .as_ref()
                .map(|es| es.as_dyn_envelope()),
        )
        .pitch_envelope(spec.pitch_envelope.as_ref().map(|es| es.as_dyn_envelope()));
    if let Some(base_frequency) = spec.base_frequency {
        generator = generator.frequency(base_frequency);
    }
    let generator = generator.build();

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
            (*voices).max(1),
            sample_rate,
            allocation.clone(),
        )),
    };

    let mut compiled_system = System::new();
    let source_index = compiled_system.add_source(system_source);
    let sink_index = compiled_system.add_sink(Box::new(SimpleSink::new()));

    // Serial fx chain: source → fx[0] → … → fx[n-1] → gain → sink.
    // The gain stage is always present, so there is a single wiring shape.
    let mut previous: Option<NodeIndex<u32>> = None;
    for fx in spec.fx.iter() {
        let mut filter = create_filter(&fx.type_id, sample_rate)?;
        for (param, value) in fx.params.iter() {
            filter.set_parameter(param, *value);
        }

        let filter_index = compiled_system.add_filter(filter);
        match previous {
            None => compiled_system.connect_source(source_index, filter_index, 0),
            Some(previous_index) => {
                compiled_system.connect(previous_index, filter_index, 0, 0);
            }
        }
        previous = Some(filter_index);
    }

    let gain_index = compiled_system.add_filter(Box::new(GainFilter::new(spec.gain)));
    match previous {
        None => compiled_system.connect_source(source_index, gain_index, 0),
        Some(previous_index) => {
            compiled_system.connect(previous_index, gain_index, 0, 0);
        }
    }
    compiled_system.connect_sink(gain_index, sink_index, 0);

    compiled_system
        .compute()
        .map_err(|e| SpecError::Compute(format!("{e:?}")))?;

    Ok(compiled_system)
}

/// Bridges an [`InstrumentSpec`] into the [`Instrument`]-slot world of
/// [`AudioGraph`](crate::app::audio_graph::AudioGraph) until slots hold specs
/// directly (spec migration step 3).
///
/// Only the graph path is implemented: the legacy `tick()`/`get_output()`
/// engine never sees spec instruments.
#[derive(Debug, Clone)]
pub struct SpecInstrument {
    pub spec: InstrumentSpec,
}

impl SpecInstrument {
    /// Wraps a spec, validating it eagerly so `as_system()` cannot fail later.
    pub fn new(spec: InstrumentSpec) -> Result<Self, SpecError> {
        validate_spec(&spec)?;
        Ok(Self { spec })
    }
}

impl Instrument for SpecInstrument {
    fn start_note(&mut self, _note: Note, _velocity: f32) {}

    fn stop_note(&mut self, _note: Note) {}

    fn get_output(&mut self) -> f32 {
        0.0
    }

    fn tick(&mut self) {}

    fn as_system(&self, sample_rate: f32) -> System {
        compile_spec(&self.spec, sample_rate)
            .expect("SpecInstrument was validated at construction; compile_spec cannot fail")
    }
}
