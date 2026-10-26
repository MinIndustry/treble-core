use crate::core::{
    Block, Note,
    filters::prelude::*,
    generator::prelude::*,
    graph::{Filter, Source},
};
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

#[derive(Clone, Debug, ModuleMetadata, ModuleParameters)]
#[treble(module(
    id = "treble.oscillator",
    version = 1,
    name = "Oscillator",
    category = "Sources"
))]
#[treble(output(id = "out", index = 0, signal = audio(channels = 2)))]
pub(crate) struct Oscillator {
    #[treble(param(id = "frequency", default = 440.0, range(min = 1.0, max = 20000.0), unit = hz, scale = logarithmic))]
    frequency: f32,
    #[treble(param(id = "amplitude", default = 0.2, range(min = 0.0, max = 1.0)))]
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

    fn retune(&mut self) {
        let frequency = if self.follow_keys {
            self.held
                .last()
                .map_or(self.frequency, |(note, _)| note.frequency())
        } else {
            self.frequency
        };
        self.generator
            .set_frequency(frequency.min(self.sample_rate * 0.49));
    }
}

impl Source for Oscillator {
    fn pull(&mut self, frames: usize) -> Block {
        let gain = if self.drone {
            self.amplitude
        } else {
            self.held
                .last()
                .map_or(0.0, |(_, velocity)| self.amplitude * velocity)
        };
        if gain == 0.0 {
            return vec![[0.0; 2]; frames];
        }
        (0..frames)
            .map(|_| [self.generator.tick(1.0 / self.sample_rate) * gain; 2])
            .collect()
    }
    fn start(&mut self) {
        self.drone = true;
        self.generator.start();
        self.retune();
    }
    fn stop(&mut self) {
        self.drone = false;
        self.held.clear();
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
        self.held.retain(|(held, _)| *held != note);
        self.retune();
    }
    fn is_active(&self) -> bool {
        self.drone || !self.held.is_empty()
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
    pub(crate) cutoff: f32,
    #[treble(param(id = "resonance", default = 0.70710677, range(min = 0.5, max = 20.0), unit = q))]
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
    pub(crate) level: f32,
}

/// Built-in playable contracts. No unsupported ports are advertised.
#[derive(Clone, Debug)]
pub struct ModuleRegistry {
    descriptors: Vec<ModuleDescriptor>,
}

impl ModuleRegistry {
    pub fn builtins() -> Self {
        Self {
            descriptors: vec![
                Oscillator::descriptor(),
                LowPass::descriptor(),
                Delay::descriptor(),
                Output::descriptor(),
                NoteInput::descriptor(),
            ],
        }
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
            parameter.validate_value(value)?;
        }
        match instance.type_id.as_str() {
            "treble.note_input" => Ok(RuntimeModule::NoteInput),
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
            _ => Err(super::error("type_id", "no installed runtime factory")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
