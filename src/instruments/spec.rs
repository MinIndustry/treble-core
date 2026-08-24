//! Spec module
//! This module defines the specification for custom instruments.
//! The aim of this module is to slowly replace the hardcoded instruments

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock, Weak};

use petgraph::graph::NodeIndex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
    #[error("Sample error: {0}")]
    Sample(String),
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
    /// When both fields are present for compatibility with an older editor,
    /// the explicit fixed `frequency` takes precedence.
    pub frequency_relation: Option<FrequencyRelation>,
    /// Fixed tone frequency in Hz. `None` — the builder default.
    pub frequency: Option<f32>,
    pub amplitude_envelope: Option<EnvelopeSpec>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SampleSpec {
    pub path: PathBuf,
    #[serde(default = "default_root_midi")]
    pub root_midi: u8,
    #[serde(default)]
    pub start_seconds: f32,
    pub end_seconds: Option<f32>,
    #[serde(default)]
    pub looped: bool,
}

fn default_root_midi() -> u8 {
    60
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

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum NoteLifecycle {
    /// Ignore external note-off and complete the instrument's natural envelope.
    OneShot,
    /// Hold while pressed, then enter the envelope's release stage.
    #[default]
    Gated,
    /// Silence the affected voice immediately on note-off.
    Cutoff,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstrumentSpec {
    pub name: String,
    #[serde(default)]
    pub note_lifecycle: NoteLifecycle,
    pub voice: VoiceSpec,
    pub tones: Vec<ToneSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample: Option<SampleSpec>,
    pub mix_mode: MixMode,
    pub pitch_envelope: Option<EnvelopeSpec>,
    pub amplitude_envelope: Option<EnvelopeSpec>,
    pub base_frequency: Option<f32>,
    pub fx: Vec<FxSpec>,
    pub gain: f32,
    pub velocity_sensitivity: f32,
    pub mods: Vec<ModSpec>,
}

fn segment_duration(segment: &SegmentSpec) -> f32 {
    match segment {
        SegmentSpec::Linear { duration, .. } | SegmentSpec::Bezier { duration, .. } => *duration,
        SegmentSpec::Constant { duration, .. } => duration.unwrap_or(0.0),
    }
}

fn envelope_timing(envelope: &EnvelopeSpec) -> (f32, f32) {
    match envelope {
        EnvelopeSpec::Adsr {
            attack,
            decay,
            release,
            ..
        } => (*attack + *decay, *release),
        EnvelopeSpec::Segments {
            attack,
            decay,
            sustain,
            release,
        } => (
            segment_duration(attack)
                + segment_duration(decay)
                + sustain.as_ref().map(segment_duration).unwrap_or(0.0),
            segment_duration(release),
        ),
        EnvelopeSpec::Segment(segment) => (segment_duration(segment), 0.0),
    }
}

fn natural_note_timing(spec: &InstrumentSpec) -> (f32, f32) {
    if let Some(envelope) = &spec.amplitude_envelope {
        return envelope_timing(envelope);
    }
    spec.tones
        .iter()
        .filter_map(|tone| tone.amplitude_envelope.as_ref())
        .map(envelope_timing)
        .max_by(|left, right| (left.0 + left.1).total_cmp(&(right.0 + right.1)))
        .unwrap_or((0.25, 0.01))
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
    if let Some(sample) = &spec.sample {
        if sample.root_midi > 127 {
            return Err(SpecError::Sample(
                "root_midi must be between 0 and 127".into(),
            ));
        }
        if !sample.start_seconds.is_finite() || sample.start_seconds < 0.0 {
            return Err(SpecError::Sample(
                "start_seconds must be a finite positive value".into(),
            ));
        }
        if sample
            .end_seconds
            .is_some_and(|end| !end.is_finite() || end <= sample.start_seconds)
        {
            return Err(SpecError::Sample(
                "end_seconds must be finite and greater than start_seconds".into(),
            ));
        }
        hound::WavReader::open(&sample.path).map_err(|error| {
            SpecError::Sample(format!(
                "could not open '{}': {error}",
                sample.path.display()
            ))
        })?;
    }
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

pub(crate) fn create_filter(
    node_type: &str,
    sample_rate: f32,
) -> Result<Box<dyn Filter>, SpecError> {
    for entry in inventory::iter::<crate::meta::FilterRegistration>() {
        let info = (entry.info)();
        if info.type_id == node_type {
            let mut filter = (entry.create)();
            // Filters that expose sample_rate as a parameter pick up the engine rate here.
            if filter.supports_parameter("sample_rate") {
                debug_assert!(filter.set_parameter("sample_rate", sample_rate));
            }
            return Ok(filter);
        }
    }
    Err(SpecError::UnknownFilter(node_type.to_string()))
}

#[derive(Debug)]
struct SampleData {
    frames: Vec<crate::core::Frame>,
    sample_rate: u32,
}

static SAMPLE_CACHE: OnceLock<Mutex<HashMap<PathBuf, Weak<SampleData>>>> = OnceLock::new();

fn load_sample(spec: &SampleSpec) -> Result<Arc<SampleData>, SpecError> {
    let path = std::fs::canonicalize(&spec.path).map_err(|error| {
        SpecError::Sample(format!(
            "could not resolve '{}': {error}",
            spec.path.display()
        ))
    })?;
    let cache = SAMPLE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(data) = cache
        .lock()
        .expect("sample cache lock poisoned")
        .get(&path)
        .and_then(Weak::upgrade)
    {
        return Ok(data);
    }

    let mut reader = hound::WavReader::open(&path).map_err(|error| {
        SpecError::Sample(format!("could not decode '{}': {error}", path.display()))
    })?;
    let format = reader.spec();
    let channels = format.channels.max(1) as usize;
    let values = match format.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(|sample| sample.map(|value| value.clamp(-1.0, 1.0)))
            .collect::<Result<Vec<_>, _>>(),
        hound::SampleFormat::Int => {
            let denominator = 2_f32.powi(format.bits_per_sample.saturating_sub(1) as i32);
            reader
                .samples::<i32>()
                .map(|sample| sample.map(|value| value as f32 / denominator))
                .collect::<Result<Vec<_>, _>>()
        }
    }
    .map_err(|error| SpecError::Sample(format!("invalid WAV samples: {error}")))?;
    let start = (spec.start_seconds * format.sample_rate as f32).round() as usize;
    let end = spec
        .end_seconds
        .map(|seconds| (seconds * format.sample_rate as f32).round() as usize)
        .unwrap_or(values.len() / channels)
        .min(values.len() / channels);
    if start >= end {
        return Err(SpecError::Sample(
            "the selected sample range contains no audio".into(),
        ));
    }
    let frames = (start..end)
        .map(|frame| {
            let offset = frame * channels;
            let left = values[offset];
            let right = if channels > 1 {
                values[offset + 1]
            } else {
                left
            };
            [left, right]
        })
        .collect();
    let data = Arc::new(SampleData {
        frames,
        sample_rate: format.sample_rate,
    });
    cache
        .lock()
        .expect("sample cache lock poisoned")
        .insert(path, Arc::downgrade(&data));
    Ok(data)
}

#[derive(Debug, Clone)]
struct SampleVoice {
    note: crate::Note,
    cursor: f64,
    increment: f64,
    velocity: f32,
    release_frames: Option<usize>,
}

#[derive(Debug, Clone)]
struct SampleSource {
    data: Arc<SampleData>,
    root_midi: u8,
    looped: bool,
    output_sample_rate: f32,
    max_voices: usize,
    voices: Vec<SampleVoice>,
}

impl SampleSource {
    fn new(
        data: Arc<SampleData>,
        root_midi: u8,
        looped: bool,
        output_sample_rate: f32,
        max_voices: usize,
    ) -> Self {
        Self {
            data,
            root_midi,
            looped,
            output_sample_rate,
            max_voices,
            voices: Vec::new(),
        }
    }

    fn release(&mut self, note: Option<crate::Note>) {
        let fade = (self.output_sample_rate * 0.005).round().max(1.0) as usize;
        for voice in &mut self.voices {
            if note.is_none_or(|note| voice.note == note) {
                voice.release_frames = Some(fade);
            }
        }
    }
}

impl Source for SampleSource {
    fn pull(&mut self, block_size: usize) -> crate::core::Block {
        let mut output = crate::core::audio::silent_block(block_size);
        let sample_len = self.data.frames.len();
        for frame in &mut output {
            for voice in &mut self.voices {
                if sample_len == 0 {
                    continue;
                }
                if voice.cursor >= sample_len as f64 {
                    if self.looped {
                        voice.cursor %= sample_len as f64;
                    } else {
                        continue;
                    }
                }
                let index = voice.cursor.floor() as usize;
                let next = (index + 1).min(sample_len - 1);
                let fraction = (voice.cursor - index as f64) as f32;
                let gain = voice
                    .release_frames
                    .map(|remaining| remaining as f32 / (self.output_sample_rate * 0.005).max(1.0))
                    .unwrap_or(1.0)
                    * voice.velocity;
                for (channel, output) in frame.iter_mut().enumerate() {
                    let value = self.data.frames[index][channel]
                        + (self.data.frames[next][channel] - self.data.frames[index][channel])
                            * fraction;
                    *output += value * gain;
                }
                voice.cursor += voice.increment;
                if let Some(remaining) = &mut voice.release_frames {
                    *remaining = remaining.saturating_sub(1);
                }
            }
            self.voices.retain(|voice| {
                voice.release_frames != Some(0) && (self.looped || voice.cursor < sample_len as f64)
            });
        }
        output
    }

    fn stop(&mut self) {
        self.release(None);
    }

    fn kill(&mut self) {
        self.release(None);
    }

    fn start_note(&mut self, note: crate::Note, velocity: f32) {
        self.voices.retain(|voice| voice.note != note);
        if self.voices.len() >= self.max_voices {
            self.voices.remove(0);
        }
        let pitch_ratio = 2_f64.powf((note.to_midi() as f64 - self.root_midi as f64) / 12.0);
        self.voices.push(SampleVoice {
            note,
            cursor: 0.0,
            increment: self.data.sample_rate as f64 / self.output_sample_rate as f64 * pitch_ratio,
            velocity,
            release_frames: None,
        });
    }

    fn stop_note(&mut self, note: crate::Note) {
        self.release(Some(note));
    }

    fn kill_note(&mut self, note: crate::Note) {
        self.release(Some(note));
    }

    fn is_active(&self) -> bool {
        !self.voices.is_empty()
    }
}

pub fn compile_spec(spec: &InstrumentSpec, sample_rate: f32) -> Result<System, SpecError> {
    validate_spec(spec)?;
    let (voice_source, release_after, release_duration): (Box<dyn Source>, f32, f32) =
        if let Some(sample) = &spec.sample {
            let data = load_sample(sample)?;
            let duration = data.frames.len() as f32 / data.sample_rate as f32;
            let max_voices = match spec.voice {
                VoiceSpec::Mono { .. } => 1,
                VoiceSpec::Poly { voices, .. } => voices.max(1),
            };
            (
                Box::new(SampleSource::new(
                    data,
                    sample.root_midi,
                    sample.looped,
                    sample_rate,
                    max_voices,
                )),
                duration,
                0.005,
            )
        } else {
            let mut generator = MultiToneGeneratorBuilder::new();
            for tone in &spec.tones {
                let mut tone_builder = ToneGeneratorBuilder::new().waveform(tone.waveform.clone());
                if let Some(frequency) = tone.frequency {
                    tone_builder = tone_builder.frequency(frequency);
                } else if let Some(relation) = &tone.frequency_relation {
                    tone_builder = tone_builder.frequency_relation(relation.clone());
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

            let voice_source: Box<dyn Source> = match &spec.voice {
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
            let (release_after, release_duration) = natural_note_timing(spec);
            (voice_source, release_after, release_duration)
        };
    let system_source: Box<dyn Source> = Box::new(crate::core::graph::NoteLifecycleSource::new(
        voice_source,
        spec.note_lifecycle,
        sample_rate,
        release_after,
        release_duration,
    ));

    let mut compiled_system = System::new();
    let source_index = compiled_system.add_source(system_source);
    let sink_index = compiled_system.add_sink(Box::new(SimpleSink::new()));

    // Serial fx chain: source → fx[0] → … → fx[n-1] → gain → sink.
    // The gain stage is always present, so there is a single wiring shape.
    let mut previous: Option<NodeIndex<u32>> = None;
    for fx in spec.fx.iter() {
        let mut filter = create_filter(&fx.type_id, sample_rate)?;
        for (param, value) in fx.params.iter() {
            if !filter.set_parameter(param, *value) {
                return Err(SpecError::UnknownParameter {
                    filter: fx.type_id.clone(),
                    param: param.clone(),
                });
            }
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
