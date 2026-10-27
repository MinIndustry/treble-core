use crate::core::graph::{Entry, ParameterModulation, bind_modulation};
use crate::core::{
    Block, Note,
    filters::prelude::*,
    generator::prelude::*,
    graph::{Filter, Source},
};
use std::sync::Arc;
use treble_derive::{ModuleMetadata, ModuleParameters, ParameterEnum};
use treble_meta::{
    MetaFilter, ModuleDescriptor, ModuleInstance, ModuleMetadata as _, ParameterValue,
    ValidationErrors,
};

pub enum RuntimeModule {
    Source(Box<dyn Source>),
    Filter(Box<dyn Filter>),
    /// Event routing only; no fake audio source is installed.
    NoteInput,
}

// All built-in float configuration fields use f32. Match ModuleParameters:
// convert first, then validate the representable value against quantized bounds.
// Keep the user's saved base value; this conversion is only for execution checks.
pub(crate) fn runtime_value(value: &ParameterValue) -> ParameterValue {
    match value {
        ParameterValue::Float(value) => ParameterValue::Float(f64::from(*value as f32)),
        value => value.clone(),
    }
}

#[derive(ModuleMetadata)]
#[treble(module(
    id = "treble.note_input",
    version = 1,
    name = "Note input",
    category = "Control"
))]
#[treble(output(id = "out", index = 0, signal = note))]
struct NoteInput {
    #[treble(input(id = "in", index = 0, signal = note))]
    _input: (),
}

#[derive(Clone, Debug, ParameterEnum)]
enum Shape {
    #[treble(value(id = "sine"))]
    Sine,
    #[treble(value(id = "saw"))]
    Saw,
    #[treble(value(id = "square"))]
    Square,
    #[treble(value(id = "triangle"))]
    Triangle,
    #[treble(value(id = "white_noise"))]
    WhiteNoise,
    #[treble(value(id = "pink_noise"))]
    PinkNoise,
}

impl Shape {
    fn waveform(&self) -> Waveform {
        match self {
            Self::Sine => Waveform::Sine,
            Self::Saw => Waveform::Sawtooth,
            Self::Square => Waveform::Square,
            Self::Triangle => Waveform::Triangle,
            Self::WhiteNoise => Waveform::WhiteNoise,
            Self::PinkNoise => Waveform::PinkNoise,
        }
    }
}

#[derive(Clone, Debug, ParameterEnum)]
enum GateBehavior {
    #[treble(value(id = "note"))]
    Note,
    #[treble(value(id = "envelope"))]
    Envelope,
}

#[derive(Clone, Debug, ModuleMetadata, ModuleParameters)]
#[treble(module(
    id = "treble.oscillator",
    version = 1,
    name = "Oscillator",
    category = "Sources"
))]
#[treble(output(id = "out", index = 0, signal = audio(channels = 2)))]
pub(crate) struct Oscillator {
    #[treble(param(id = "gate_mode", default = GateBehavior::Note, update = rebuild))]
    gate_mode: GateBehavior,
    released: Option<(Note, f32)>,
    #[treble(param(id = "frequency", default = 440.0, range(min = 1.0, max = 20000.0), unit = hz, scale = logarithmic))]
    #[treble(modulation(port = "frequency_cv", index = 1, modes = [add, multiply, replace], mapping = native, depth = 100.0, rate = sample, connections = sum))]
    frequency: f32,
    #[treble(param(id = "amplitude", default = 0.2, range(min = 0.0, max = 1.0)))]
    #[treble(modulation(port = "amplitude_cv", index = 2, modes = [add, multiply, replace], mapping = native, rate = sample, connections = sum))]
    amplitude: f32,
    #[treble(param(id = "follow_keys", default = true))]
    follow_keys: bool,
    #[treble(param(id = "waveform", default = Shape::Sine, update = rebuild))]
    waveform: Shape,
    #[treble(input(id = "trigger", index = 0, signal = note))]
    _trigger: (),
    #[treble(context(sample_rate))]
    sample_rate: f32,
    generator: SingleToneGenerator,
    held: Vec<(Note, f32)>,
    drone: bool,
}

impl Oscillator {
    pub(crate) fn new(sample_rate: f32) -> Self {
        Self {
            gate_mode: GateBehavior::Note,
            released: None,
            frequency: 440.0,
            amplitude: 0.2,
            follow_keys: true,
            waveform: Shape::Sine,
            _trigger: (),
            sample_rate,
            generator: builder::ToneGeneratorBuilder::new().build(),
            held: Vec::new(),
            drone: false,
        }
    }

    pub(crate) fn configure(
        &mut self,
        id: &str,
        value: ParameterValue,
    ) -> Result<(), ValidationErrors> {
        self.set_parameter_value(id, value)?;
        self.generator = builder::ToneGeneratorBuilder::new()
            .frequency(self.frequency)
            .waveform(self.waveform.waveform())
            .build();
        Ok(())
    }

    fn frequency_base(&self) -> f32 {
        if self.follow_keys {
            self.held
                .last()
                .or({
                    if matches!(self.gate_mode, GateBehavior::Envelope) {
                        self.released.as_ref()
                    } else {
                        None
                    }
                })
                .map_or(self.frequency, |(note, _)| note.frequency())
        } else {
            self.frequency
        }
    }

    fn retune(&mut self) {
        let frequency = self.frequency_base();
        self.generator
            .set_frequency(frequency.min(self.sample_rate * 0.49));
    }
}

impl Source for Oscillator {
    fn pull(&mut self, frames: usize) -> Block {
        self.pull_modulated(frames, &[])
    }
    fn pull_modulated(&mut self, frames: usize, inputs: &[ParameterModulation]) -> Block {
        let velocity = if self.drone {
            1.0
        } else {
            self.held
                .last()
                .or({
                    if matches!(self.gate_mode, GateBehavior::Envelope) {
                        self.released.as_ref()
                    } else {
                        None
                    }
                })
                .map_or(0.0, |(_, velocity)| *velocity)
        };
        if velocity == 0.0 || (inputs.is_empty() && self.amplitude == 0.0) {
            return vec![[0.0; 2]; frames];
        }
        let base_frequency = self.frequency_base();
        let frequency = bind_modulation(inputs, "frequency", base_frequency);
        let amplitude = bind_modulation(inputs, "amplitude", self.amplitude);
        let mut output = Vec::with_capacity(frames);
        for index in 0..frames {
            let frequency = frequency.value(index).clamp(1.0, self.sample_rate * 0.49);
            let amplitude = amplitude.value(index);
            self.generator.set_frequency(frequency);
            output.push([self.generator.tick(1.0 / self.sample_rate) * amplitude * velocity; 2]);
        }
        self.retune();
        output
    }
    fn start(&mut self) {
        self.drone = true;
        self.released = None;
        self.generator.start();
        self.retune();
    }
    fn stop(&mut self) {
        self.drone = false;
        self.held.clear();
        self.released = None;
    }
    fn kill(&mut self) {
        self.stop();
    }
    fn start_note(&mut self, note: Note, velocity: f32) {
        self.held.retain(|(held, _)| *held != note);
        self.held.push((note, velocity));
        if self.held.len() == 1 {
            self.generator.start();
        }
        self.retune();
    }
    fn stop_note(&mut self, note: Note) {
        if let Some(held) = self.held.iter().find(|(held, _)| *held == note).copied() {
            self.released = Some(held);
        }
        self.held.retain(|(held, _)| *held != note);
        self.retune();
    }
    fn is_active(&self) -> bool {
        self.drone
            || !self.held.is_empty()
            || (matches!(self.gate_mode, GateBehavior::Envelope) && self.released.is_some())
    }
    fn supports_parameter(&self, name: &str) -> bool {
        matches!(name, "frequency" | "amplitude" | "follow_keys")
    }
    fn set_parameter(&mut self, name: &str, value: f32) -> bool {
        match name {
            "frequency" => self.frequency = value,
            "amplitude" => self.amplitude = value,
            "follow_keys" => self.follow_keys = value != 0.0,
            _ => return false,
        }
        self.retune();
        true
    }
}

#[derive(ModuleMetadata, ModuleParameters)]
#[treble(module(
    id = "treble.low_pass",
    version = 1,
    name = "Low-pass",
    category = "Filters"
))]
#[treble(output(id = "out", index = 0, signal = audio(channels = 2)))]
pub(crate) struct LowPass {
    #[treble(input(id = "in", index = 0, signal = audio(channels = 2), connections = sum))]
    pub(crate) _input: (),
    #[treble(param(id = "cutoff", default = 1000.0, range(min = 1.0, max = 20000.0), unit = hz, scale = logarithmic))]
    #[treble(modulation(port = "cutoff_cv", modes = [add, multiply, replace], mapping = octaves, rate = sample, connections = sum))]
    pub(crate) cutoff: f32,
    #[treble(param(id = "resonance", default = 0.70710677, range(min = 0.5, max = 20.0), unit = q))]
    #[treble(modulation(port = "resonance_cv", modes = [add, multiply, replace], mapping = native, rate = sample, connections = sum))]
    pub(crate) resonance: f32,
}

#[derive(ModuleMetadata, ModuleParameters)]
#[treble(module(id = "treble.delay", version = 1, name = "Delay", category = "Effects"))]
#[treble(output(id = "out", index = 0, signal = audio(channels = 2)))]
pub(crate) struct Delay {
    #[treble(input(id = "in", index = 0, signal = audio(channels = 2), connections = sum))]
    pub(crate) _input: (),
    // Existing delay buffer resizing allocates. Require a prepared replacement.
    #[treble(param(id = "time", default = 0.25, range(min = 0.001, max = 2.0), unit = seconds, update = rebuild))]
    pub(crate) time: f32,
    #[treble(param(id = "feedback", default = 0.0, range(min = 0.0, max = 0.99)))]
    pub(crate) feedback: f32,
    #[treble(param(id = "mix", default = 0.5, range(min = 0.0, max = 1.0)))]
    pub(crate) mix: f32,
}

#[derive(ModuleMetadata, ModuleParameters)]
#[treble(module(
    id = "treble.output",
    version = 1,
    name = "Output",
    category = "Output"
))]
#[treble(output(id = "out", index = 0, signal = audio(channels = 2)))]
pub(crate) struct Output {
    #[treble(input(id = "in", index = 0, signal = audio(channels = 2), connections = sum))]
    pub(crate) _input: (),
    #[treble(param(id = "level", default = 0.8, range(min = 0.0, max = 1.0)))]
    #[treble(modulation(port = "level_cv", modes = [add, multiply, replace], mapping = normalized, rate = sample, connections = sum))]
    pub(crate) level: f32,
}

/// A continuously running bipolar sample control oscillator (including audio-rate FM).
#[derive(Clone, Debug, ModuleMetadata, ModuleParameters)]
#[treble(module(id = "treble.lfo", version = 1, name = "LFO", category = "Control"))]
#[treble(output(id = "out", index = 0, signal = control(domain = bipolar, rate = sample)))]
struct Lfo {
    #[treble(param(id = "frequency", default = 2.0, range(min = 0.001, max = 20000.0), unit = hz, scale = logarithmic))]
    #[treble(modulation(port = "frequency_cv", modes = [add, multiply, replace], mapping = octaves, rate = sample, connections = sum))]
    frequency: f32,
    #[treble(param(id = "amplitude", default = 1.0, range(min = 0.0, max = 1.0)))]
    #[treble(modulation(port = "amplitude_cv", modes = [add, multiply, replace], mapping = native, rate = sample, connections = sum))]
    amplitude: f32,
    #[treble(context(sample_rate))]
    sample_rate: f32,
    phase: f64,
}
impl Source for Lfo {
    fn pull(&mut self, frames: usize) -> Block {
        self.pull_modulated(frames, &[])
    }
    fn pull_modulated(&mut self, frames: usize, inputs: &[ParameterModulation]) -> Block {
        let frequency = bind_modulation(inputs, "frequency", self.frequency);
        let amplitude = bind_modulation(inputs, "amplitude", self.amplitude);
        let mut output = Vec::with_capacity(frames);
        for index in 0..frames {
            let frequency = frequency.value(index).min(self.sample_rate * 0.49);
            let amplitude = amplitude.value(index);
            output.push([(self.phase * std::f64::consts::TAU).sin() as f32 * amplitude; 2]);
            self.phase = (self.phase + f64::from(frequency) / f64::from(self.sample_rate)).fract();
        }
        output
    }
    fn is_active(&self) -> bool {
        true
    }
    fn supports_parameter(&self, name: &str) -> bool {
        matches!(name, "frequency" | "amplitude")
    }
    fn set_parameter(&mut self, name: &str, value: f32) -> bool {
        match name {
            "frequency" => self.frequency = value,
            "amplitude" => self.amplitude = value,
            _ => return false,
        }
        true
    }
}

/// Explicit per-frame mean(L,R) conversion, bounded to bipolar control range.
#[derive(Clone, Debug, ModuleMetadata)]
#[treble(module(
    id = "treble.audio_to_control",
    version = 1,
    name = "Audio to control",
    category = "Control"
))]
#[treble(output(id = "out", index = 0, signal = control(domain = bipolar, rate = sample)))]
struct AudioToControl {
    #[treble(input(id = "in", index = 0, signal = audio(channels = 2), connections = sum))]
    input: Arc<Block>,
}
impl Entry for AudioToControl {
    fn push(&mut self, block: Arc<Block>, _port: usize) {
        self.input = block;
    }
}
impl std::fmt::Display for AudioToControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Audio to control")
    }
}
impl MetaFilter for AudioToControl {
    fn metadata() -> treble_meta::FilterInfo {
        treble_meta::FilterInfo {
            name: "Audio to control",
            type_id: "treble.audio_to_control",
            description: "Per-sample stereo mean, clamped to bipolar control",
            inputs: vec![treble_meta::FilterInput {
                label: Some("Input"),
                parameter: None,
            }],
            outputs: 1,
        }
    }
}
impl Filter for AudioToControl {
    fn transform(&mut self) -> Vec<Block> {
        let output = self
            .input
            .iter()
            .map(|frame| {
                let value = frame[0] * 0.5 + frame[1] * 0.5;
                [if value.is_finite() {
                    value.clamp(-1.0, 1.0)
                } else {
                    0.0
                }; 2]
            })
            .collect();
        self.input = Arc::new(Vec::new());
        vec![output]
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Built-in playable contracts. No unsupported ports are advertised.
#[derive(Clone, Debug)]
pub struct ModuleRegistry {
    descriptors: Vec<ModuleDescriptor>,
}

impl ModuleRegistry {
    pub fn builtins() -> Self {
        let mut descriptors = vec![
            Oscillator::descriptor(),
            LowPass::descriptor(),
            Delay::descriptor(),
            Output::descriptor(),
            NoteInput::descriptor(),
            Lfo::descriptor(),
            AudioToControl::descriptor(),
            super::envelope::Adsr::descriptor(),
        ];
        descriptors.extend(super::catalog::descriptors());
        Self { descriptors }
    }
    pub fn descriptors(&self) -> &[ModuleDescriptor] {
        &self.descriptors
    }
    pub fn descriptor(&self, type_id: &str, version: u32) -> Option<&ModuleDescriptor> {
        self.descriptors
            .iter()
            .find(|d| d.type_id == type_id && d.version == version)
    }

    pub fn instantiate(
        &self,
        instance: &ModuleInstance,
        sample_rate: f32,
    ) -> Result<RuntimeModule, ValidationErrors> {
        if !sample_rate.is_finite() || !(8000.0..=192000.0).contains(&sample_rate) {
            return Err(super::error("sample_rate", "expected 8000..=192000 Hz"));
        }
        let descriptor = self
            .descriptor(&instance.type_id, instance.type_version)
            .ok_or_else(|| super::error("type_id", "unknown module type or version"))?;
        for (id, value) in &instance.parameters {
            let parameter = descriptor
                .parameters
                .iter()
                .find(|p| p.id == *id)
                .ok_or_else(|| super::error(format!("parameters.{id}"), "unknown parameter"))?;
            parameter.validate_value(&runtime_value(value))?;
        }
        match instance.type_id.as_str() {
            "treble.note_input" => Ok(RuntimeModule::NoteInput),
            "treble.adsr" => {
                let mut envelope = super::envelope::Adsr::new(sample_rate);
                for (id, value) in &instance.parameters {
                    envelope.set_parameter_value(id, runtime_value(value))?;
                }
                Ok(RuntimeModule::Source(Box::new(envelope)))
            }
            "treble.lfo" => {
                let mut lfo = Lfo {
                    frequency: 2.0,
                    amplitude: 1.0,
                    sample_rate,
                    phase: 0.0,
                };
                for (id, value) in &instance.parameters {
                    lfo.set_parameter_value(id, value.clone())?;
                }
                Ok(RuntimeModule::Source(Box::new(lfo)))
            }
            "treble.audio_to_control" => Ok(RuntimeModule::Filter(Box::new(AudioToControl {
                input: Arc::new(Vec::new()),
            }))),
            "treble.oscillator" => {
                let mut config = Oscillator::new(sample_rate);
                for (id, value) in &instance.parameters {
                    config.configure(id, value.clone())?;
                }
                Ok(RuntimeModule::Source(Box::new(config)))
            }
            "treble.low_pass" => {
                let mut config = LowPass {
                    _input: (),
                    cutoff: 1000.0,
                    resonance: 0.70710677,
                };
                for (id, value) in &instance.parameters {
                    config.set_parameter_value(id, value.clone())?;
                }
                Ok(RuntimeModule::Filter(Box::new(
                    LowPassFilter::with_resonance(config.cutoff, config.resonance, sample_rate),
                )))
            }
            "treble.delay" => {
                let mut config = Delay {
                    _input: (),
                    time: 0.25,
                    feedback: 0.0,
                    mix: 0.5,
                };
                for (id, value) in &instance.parameters {
                    config.set_parameter_value(id, value.clone())?;
                }
                let mut filter = DelayFilter::new(sample_rate, config.time);
                filter.set_parameter("feedback", config.feedback);
                filter.set_parameter("mix", config.mix);
                Ok(RuntimeModule::Filter(Box::new(filter)))
            }
            "treble.output" => {
                let mut config = Output {
                    _input: (),
                    level: 0.8,
                };
                for (id, value) in &instance.parameters {
                    config.set_parameter_value(id, value.clone())?;
                }
                Ok(RuntimeModule::Filter(Box::new(GainFilter::new(
                    config.level,
                ))))
            }
            _ => super::catalog::instantiate(instance, sample_rate, descriptor),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn modulation(parameter: &str, mapping: treble_meta::ModulationMapping) -> ParameterModulation {
        ParameterModulation {
            parameter: parameter.into(),
            mode: treble_meta::ModulationMode::Add,
            mapping,
            domain: None,
            signals: vec![(
                Arc::new(vec![[-0.5; 2], [-0.25; 2], [0.0; 2], [0.25; 2], [0.5; 2]]),
                1.0,
            )],
        }
    }
    #[test]
    fn envelope_carrier_retains_pitch_and_velocity_after_note_off_but_panic_silences_it() {
        let mut oscillator = Oscillator::new(48000.0);
        oscillator
            .configure("gate_mode", ParameterValue::Enum("envelope".into()))
            .unwrap();
        let note = Note::from_midi(60);
        oscillator.start_note(note, 0.5);
        oscillator.pull(100);
        oscillator.stop_note(note);
        assert!((oscillator.frequency_base() - note.frequency()).abs() < 1e-5);
        assert!(
            oscillator
                .pull(100)
                .iter()
                .flatten()
                .any(|v| v.abs() > 0.01)
        );
        oscillator.kill();
        assert!(oscillator.pull(100).iter().flatten().all(|v| *v == 0.0));
    }
    #[test]
    fn oscillator_fm_and_am_are_sample_views_with_persistent_bases() {
        let mut oscillator = Oscillator::new(48000.0);
        let mut reference = builder::ToneGeneratorBuilder::new().build();
        oscillator.start();
        reference.start();
        let inputs = vec![
            modulation("frequency", treble_meta::ModulationMapping::Octaves),
            modulation("amplitude", treble_meta::ModulationMapping::Native),
        ];
        let actual = oscillator.pull_modulated(5, &inputs);
        for (index, frame) in actual.iter().enumerate() {
            let cv = -0.5 + index as f32 * 0.25;
            reference.set_frequency(440.0 * 2.0f32.powf(cv));
            let expected = reference.tick(1.0 / 48000.0) * (0.2 + cv);
            assert!((frame[0] - expected).abs() < 1e-6);
        }
        assert_eq!(oscillator.frequency, 440.0);
        assert_eq!(oscillator.amplitude, 0.2);
        reference.set_frequency(440.0);
        assert!((oscillator.pull(1)[0][0] - reference.tick(1.0 / 48000.0) * 0.2).abs() < 1e-6);
    }
    #[test]
    fn lfo_runs_without_keys_and_converter_reduces_each_stereo_frame() {
        let mut lfo = Lfo {
            frequency: 1000.0,
            amplitude: 0.5,
            sample_rate: 48000.0,
            phase: 0.0,
        };
        let mut split = lfo.clone();
        let whole = lfo.pull(100);
        let chunks: Block = [split.pull(13), split.pull(29), split.pull(58)].concat();
        assert_eq!(whole, chunks);
        for (index, frame) in whole.iter().enumerate() {
            let expected = (index as f64 / 48.0 * std::f64::consts::TAU).sin() as f32 * 0.5;
            assert!((frame[0] - expected).abs() < 1e-6);
        }
        let mut converter = AudioToControl {
            input: Arc::new(vec![[1.0, -1.0], [0.6, 0.2], [3.0, 1.0], [f32::NAN, 0.2]]),
        };
        assert_eq!(
            converter.transform()[0],
            vec![[0.0; 2], [0.4; 2], [1.0; 2], [0.0; 2]]
        );
    }
    #[test]
    fn builtin_contracts_are_valid() {
        for descriptor in ModuleRegistry::builtins().descriptors() {
            descriptor.validate().unwrap();
        }
    }
    #[test]
    fn every_contract_has_a_factory() {
        let registry = ModuleRegistry::builtins();
        for d in registry.descriptors() {
            registry
                .instantiate(
                    &ModuleInstance {
                        id: "test".into(),
                        type_id: d.type_id.clone(),
                        type_version: d.version,
                        parameters: Default::default(),
                    },
                    48000.0,
                )
                .unwrap();
        }
    }
    #[test]
    fn oscillator_defaults_match_derived_contract_and_typed_controls() {
        let mut source = Oscillator::new(48000.0);
        for param in Oscillator::descriptor().parameters {
            assert_eq!(source.parameter_value(&param.id).unwrap(), param.default);
        }
        assert!(
            source
                .configure("follow_keys", ParameterValue::Float(1.0))
                .is_err()
        );
        source
            .configure("waveform", ParameterValue::Enum("saw".into()))
            .unwrap();
        assert!(
            source
                .configure("waveform", ParameterValue::Enum("missing".into()))
                .is_err()
        );
    }
}
