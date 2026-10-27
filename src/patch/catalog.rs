//! Registry contracts for the reusable DSP filters and sample arithmetic.
//! Defaults/ranges are declared once here; factories read these same descriptors.
use super::{RuntimeModule, error};
use crate::core::{
    filters::{
        math::{ConstantSource, MathFilter, MathOperation},
        prelude::*,
    },
    graph::Filter,
};
use treble_meta::*;

fn numeric(id: &str, default: f64, min: f64, max: f64) -> ParameterDescriptor {
    let f = |v: f64| f64::from(v as f32);
    ParameterDescriptor {
        id: id.into(),
        name: id.replace('_', " "),
        description: String::new(),
        value_type: ValueType::Float,
        default: ParameterValue::Float(f(default)),
        domain: Some(NumericDomain {
            min: f(min),
            max: f(max),
            unit: Unit::Unitless,
            scale: Scale::Linear,
            step: None,
        }),
        access: ParameterAccess::User,
        update: UpdateMode::Live,
        smoothing_ms: None,
        modulation: None,
    }
}
fn unit(mut p: ParameterDescriptor, unit: Unit, logarithmic: bool) -> ParameterDescriptor {
    let d = p.domain.as_mut().expect("numeric domain");
    d.unit = unit;
    if logarithmic {
        d.scale = Scale::Logarithmic;
    }
    p
}
fn rebuild(mut p: ParameterDescriptor) -> ParameterDescriptor {
    p.update = UpdateMode::Rebuild;
    p
}
fn enumeration(id: &str, default: &str, variants: &[&str]) -> ParameterDescriptor {
    ParameterDescriptor {
        id: id.into(),
        name: id.into(),
        description: String::new(),
        value_type: ValueType::Enum {
            variants: variants
                .iter()
                .map(|id| EnumVariant {
                    id: (*id).into(),
                    name: (*id).into(),
                })
                .collect(),
        },
        default: ParameterValue::Enum(default.into()),
        domain: None,
        access: ParameterAccess::User,
        update: UpdateMode::Rebuild,
        smoothing_ms: None,
        modulation: None,
    }
}
fn audio(channels: u16) -> SignalType {
    SignalType::Audio { channels }
}
fn control() -> SignalType {
    SignalType::Control {
        domain: ControlDomain::Bipolar,
        rate: ControlRate::Sample,
    }
}
fn port(id: &str, signal: SignalType, direction: Direction, index: u32) -> PortDescriptor {
    let connections = if direction == Direction::Output {
        ConnectionPolicy::FanOut
    } else if signal == SignalType::Gate {
        ConnectionPolicy::Single
    } else {
        ConnectionPolicy::Sum
    };
    PortDescriptor {
        id: id.into(),
        name: id.replace('_', " "),
        direction,
        index,
        signal,
        binding: PortBinding::Stream,
        connections,
    }
}
fn module(
    id: &str,
    name: &str,
    category: &str,
    inputs: Vec<(&str, SignalType)>,
    outputs: Vec<(&str, SignalType)>,
    parameters: Vec<ParameterDescriptor>,
) -> ModuleDescriptor {
    let mut ports: Vec<_> = inputs
        .into_iter()
        .enumerate()
        .map(|(i, (id, signal))| port(id, signal, Direction::Input, i as u32))
        .collect();
    ports.extend(
        outputs
            .into_iter()
            .enumerate()
            .map(|(i, (id, signal))| port(id, signal, Direction::Output, i as u32)),
    );
    ModuleDescriptor {
        type_id: format!("treble.{id}"),
        version: 1,
        name: name.into(),
        description: String::new(),
        category: category.into(),
        parameters,
        ports,
        groups: vec![],
    }
}
fn effect(
    id: &str,
    name: &str,
    category: &str,
    parameters: Vec<ParameterDescriptor>,
) -> ModuleDescriptor {
    module(
        id,
        name,
        category,
        vec![("in", audio(2))],
        vec![("out", audio(2))],
        parameters,
    )
}
fn cv(d: &mut ModuleDescriptor, parameter: &str, mapping: ModulationMapping) {
    let index = d
        .ports
        .iter()
        .filter(|p| p.direction == Direction::Input)
        .count() as u32;
    d.parameters
        .iter_mut()
        .find(|p| p.id == parameter)
        .expect("declared parameter")
        .modulation = Some(ModulationDescriptor {
        modes: vec![
            ModulationMode::Add,
            ModulationMode::Multiply,
            ModulationMode::Replace,
        ],
        default_mode: ModulationMode::Add,
        mapping,
        default_depth: 1.0,
    });
    let mut p = port(
        &format!("{parameter}_cv"),
        control(),
        Direction::Input,
        index,
    );
    p.binding = PortBinding::Parameter {
        parameter_id: parameter.into(),
    };
    d.ports.push(p);
}
pub(super) fn descriptors() -> Vec<ModuleDescriptor> {
    let hz = |id, default, min, max| unit(numeric(id, default, min, max), Unit::Hertz, true);
    let time = |id, default, min, max| unit(numeric(id, default, min, max), Unit::Seconds, true);
    let mut high = effect(
        "high_pass",
        "High-pass",
        "Filters",
        vec![
            hz("cutoff", 1000.0, 1.0, 20000.0),
            unit(numeric("resonance", 0.70710677, 0.5, 20.0), Unit::Q, false),
        ],
    );
    cv(&mut high, "cutoff", ModulationMapping::Octaves);
    cv(&mut high, "resonance", ModulationMapping::Native);
    let mut gain = effect(
        "gain",
        "Gain",
        "Math",
        vec![numeric("gain", 1.0, -16.0, 16.0)],
    );
    cv(&mut gain, "gain", ModulationMapping::Native);
    let mut pan = effect(
        "pan",
        "Pan",
        "Utility",
        vec![numeric("direction", 0.0, -1.0, 1.0)],
    );
    cv(&mut pan, "direction", ModulationMapping::Native);
    let mut size = rebuild(numeric("size", 3.0, 1.0, 4096.0));
    size.value_type = ValueType::Int;
    size.default = ParameterValue::Int(3);
    size.domain.as_mut().unwrap().step = Some(1.0);
    let mut constant = module(
        "constant",
        "Constant",
        "Sources",
        vec![],
        vec![("out", audio(2))],
        vec![numeric("value", 1.0, -16.0, 16.0)],
    );
    cv(&mut constant, "value", ModulationMapping::Native);
    let mut offset = effect(
        "offset",
        "Offset",
        "Math",
        vec![numeric("offset", 0.0, -16.0, 16.0)],
    );
    cv(&mut offset, "offset", ModulationMapping::Native);
    let mut clamp = effect(
        "clamp",
        "Clamp",
        "Math",
        vec![
            numeric("minimum", -1.0, -16.0, 16.0),
            numeric("maximum", 1.0, -16.0, 16.0),
        ],
    );
    cv(&mut clamp, "minimum", ModulationMapping::Native);
    cv(&mut clamp, "maximum", ModulationMapping::Native);
    let mut list = vec![
        high,
        gain,
        pan,
        effect(
            "band_pass",
            "Band-pass",
            "Filters",
            vec![
                rebuild(hz("low", 200.0, 1.0, 20000.0)),
                rebuild(hz("high", 5000.0, 1.0, 20000.0)),
            ],
        ),
        effect(
            "resonant_band_pass",
            "Resonant band-pass",
            "Filters",
            vec![
                hz("center_frequency", 1000.0, 20.0, 20000.0),
                unit(numeric("quality", 1.0, 0.1, 100.0), Unit::Q, false),
            ],
        ),
        effect("moving_average", "Moving average", "Filters", vec![size]),
        effect(
            "auto_pan",
            "Auto-pan",
            "Modulation",
            vec![
                unit(numeric("frequency", 0.5, 0.0, 100.0), Unit::Hertz, false),
                numeric("depth", 1.0, 0.0, 1.0),
                enumeration(
                    "waveform",
                    "sine",
                    &["sine", "triangle", "square", "saw", "random"],
                ),
            ],
        ),
        effect(
            "tremolo",
            "Tremolo",
            "Modulation",
            vec![
                unit(numeric("frequency", 1.0, 0.0, 20.0), Unit::Hertz, false),
                numeric("depth", 0.5, 0.0, 1.0),
            ],
        ),
        effect(
            "reverb",
            "Reverb",
            "Effects",
            vec![rebuild(numeric("amount", 0.3, 0.0, 1.0))],
        ),
        effect(
            "compressor",
            "Compressor",
            "Dynamics",
            vec![
                numeric("threshold", 0.5, 0.0, 1.0),
                numeric("ratio", 4.0, 1.0, 20.0),
                time("attack", 0.01, 0.0001, 0.1),
                time("release", 0.1, 0.01, 1.0),
            ],
        ),
        effect(
            "limiter",
            "Limiter",
            "Dynamics",
            vec![
                numeric("threshold", 0.95, 0.0, 1.0),
                time("attack", 0.001, 0.0001, 0.1),
                time("release", 0.2, 0.01, 1.0),
            ],
        ),
        effect(
            "clipper",
            "Clipper",
            "Dynamics",
            vec![numeric("maximum", 0.5, 0.0, 1.0)],
        ),
        constant,
        offset,
        clamp,
        effect("absolute", "Absolute", "Math", vec![]),
        module(
            "stereo_split",
            "Stereo split",
            "Routing",
            vec![("in", audio(2))],
            vec![("left", audio(1)), ("right", audio(1))],
            vec![],
        ),
        module(
            "stereo_merge",
            "Stereo merge",
            "Routing",
            vec![("left", audio(1)), ("right", audio(1))],
            vec![("out", audio(2))],
            vec![],
        ),
        module(
            "mono_to_stereo",
            "Mono to stereo",
            "Routing",
            vec![("in", audio(1))],
            vec![("out", audio(2))],
            vec![],
        ),
        module(
            "mono_to_control",
            "Mono to control",
            "Control",
            vec![("in", audio(1))],
            vec![("out", control())],
            vec![],
        ),
        module(
            "control_to_audio",
            "Control to audio",
            "Control",
            vec![("in", control())],
            vec![("out", audio(2))],
            vec![],
        ),
    ];
    list.push(module(
        "control_to_gate",
        "Control to gate",
        "Control",
        vec![("in", control())],
        vec![("out", SignalType::Gate)],
        vec![numeric("threshold", 0.0, -1.0, 1.0)],
    ));
    list.push(module(
        "gate_to_control",
        "Gate to control",
        "Control",
        vec![("in", SignalType::Gate)],
        vec![("out", control())],
        vec![],
    ));
    for (id, name) in [
        ("add", "Add"),
        ("multiply", "Multiply"),
        ("minimum", "Minimum"),
        ("maximum", "Maximum"),
    ] {
        list.push(module(
            id,
            name,
            "Math",
            vec![("a", audio(2)), ("b", audio(2))],
            vec![("out", audio(2))],
            vec![],
        ));
    }
    for d in &mut list {
        d.description = match d.type_id.as_str() {
            "treble.control_to_gate" => "Gate is high while the control signal exceeds the threshold.",
            "treble.gate_to_control" => "Expose gate samples as bipolar-compatible control values.",
            "treble.add" => "Sample-wise A + B, independently per channel. Missing inputs are zero.",
            "treble.multiply" => "Signed sample-wise A × B (ring modulation). No audio-range clipping.",
            "treble.constant" => "Continuous signed stereo DC value, independent of keyboard notes.",
            "treble.offset" => "Add a signed constant or sample-modulated offset to each channel.",
            "treble.clamp" => "Clamp each sample between the two bounds; crossing bounds are sorted.",
            "treble.stereo_split" => "Separate L/R as typed mono outputs. Internal mono blocks duplicate channel zero.",
            "treble.stereo_merge" => "Place the left and right mono inputs on their corresponding output channels.",
            "treble.mono_to_stereo" => "Duplicate a mono signal onto L/R without changing its value.",
            "treble.mono_to_control" => "Convert each mono sample to bipolar control, clamped to [-1,1].",
            "treble.control_to_audio" => "Copy each control sample to stereo audio without clipping or averaging.",
            "treble.band_pass" => "High-pass/low-pass cascade. Cutoff endpoints are sorted during preparation.",
            "treble.reverb" => "Schroeder ambience. Changing amount prepares its delay buffers off the render path.",
            _ => "Built-in sample processor. See parameter ranges and update modes.",
        }.into();
    }
    list
}
pub(super) fn scalar_binding(type_id: &str, parameter: &str) -> Option<&'static str> {
    match (type_id, parameter) {
        ("treble.control_to_gate", "threshold") => Some("threshold"),
        ("treble.high_pass", "cutoff") => Some("cutoff_frequency"),
        ("treble.high_pass", "resonance") => Some("resonance"),
        ("treble.gain", "gain") => Some("factor"),
        ("treble.pan", "direction") => Some("direction"),
        ("treble.resonant_band_pass", "center_frequency") => Some("center_frequency"),
        ("treble.resonant_band_pass", "quality") => Some("quality"),
        ("treble.auto_pan" | "treble.tremolo", "frequency") => Some("frequency"),
        ("treble.auto_pan" | "treble.tremolo", "depth") => Some("depth"),
        ("treble.compressor" | "treble.limiter", "threshold") => Some("threshold"),
        ("treble.compressor" | "treble.limiter", "attack") => Some("attack"),
        ("treble.compressor" | "treble.limiter", "release") => Some("release"),
        ("treble.compressor", "ratio") => Some("ratio"),
        ("treble.clipper", "maximum") => Some("max_ampl"),
        ("treble.constant", "value") => Some("value"),
        ("treble.offset", "offset") => Some("offset"),
        ("treble.clamp", "minimum") => Some("minimum"),
        ("treble.clamp", "maximum") => Some("maximum"),
        _ => None,
    }
}
pub(super) fn instantiate(
    instance: &ModuleInstance,
    rate: f32,
    descriptor: &ModuleDescriptor,
) -> Result<RuntimeModule, ValidationErrors> {
    let value = |id: &str| {
        instance.parameters.get(id).unwrap_or_else(|| {
            &descriptor
                .parameters
                .iter()
                .find(|p| p.id == id)
                .expect("declared parameter")
                .default
        })
    };
    let number = |id: &str| match value(id) {
        ParameterValue::Float(n) => *n as f32,
        ParameterValue::Int(n) => *n as f32,
        _ => unreachable!("validated numeric parameter"),
    };
    let id = instance.type_id.as_str();
    if id == "treble.constant" {
        return Ok(RuntimeModule::Source(Box::new(ConstantSource::new(
            number("value"),
        ))));
    }
    let mut filter: Box<dyn Filter> = match id {
        "treble.high_pass" => Box::new(HighPassFilter::with_resonance(
            number("cutoff"),
            number("resonance"),
            rate,
        )),
        "treble.gain" => Box::new(GainFilter::new(number("gain"))),
        "treble.pan" => Box::new(PanFilter::new(number("direction"))),
        "treble.band_pass" => Box::new(BandPass::new(
            number("low").min(number("high")),
            number("low").max(number("high")),
            rate,
        )),
        "treble.resonant_band_pass" => Box::new(ResonantBandpassFilter::new(
            number("center_frequency"),
            number("quality"),
            rate,
        )),
        "treble.moving_average" => Box::new(MovingAverage::new(number("size") as usize)),
        "treble.auto_pan" => {
            let ParameterValue::Enum(wave) = value("waveform") else {
                unreachable!("validated enum")
            };
            let wave = match wave.as_str() {
                "triangle" => PanWave::Triangle,
                "square" => PanWave::Square,
                "saw" => PanWave::Saw,
                "random" => PanWave::Random,
                _ => PanWave::Sine,
            };
            Box::new(AutoPanFilter::new(
                number("frequency"),
                number("depth"),
                wave,
                rate,
            ))
        }
        "treble.tremolo" => Box::new(Tremolo::new(number("frequency"), number("depth"), rate)),
        "treble.reverb" => Box::new(ReverbFilter::new(rate, number("amount"))),
        "treble.compressor" => {
            let mut f = Compressor::default();
            f.set_parameter("sample_rate", rate);
            Box::new(f)
        }
        "treble.limiter" => {
            let mut f = Limiter::default();
            f.set_parameter("sample_rate", rate);
            Box::new(f)
        }
        "treble.clipper" => Box::new(Clipper::new(number("maximum"))),
        _ => {
            let operation = match id {
                "treble.control_to_gate" => MathOperation::ControlToGate,
                "treble.gate_to_control" => MathOperation::GateToControl,
                "treble.add" => MathOperation::Add,
                "treble.multiply" => MathOperation::Multiply,
                "treble.minimum" => MathOperation::Minimum,
                "treble.maximum" => MathOperation::Maximum,
                "treble.absolute" => MathOperation::Absolute,
                "treble.offset" => MathOperation::Offset,
                "treble.clamp" => MathOperation::Clamp,
                "treble.stereo_split" => MathOperation::Split,
                "treble.stereo_merge" => MathOperation::Merge,
                "treble.mono_to_stereo" => MathOperation::MonoToStereo,
                "treble.mono_to_control" => MathOperation::MonoToControl,
                "treble.control_to_audio" => MathOperation::ControlToAudio,
                _ => return Err(error("type_id", "no installed runtime factory")),
            };
            Box::new(MathFilter::new(operation))
        }
    };
    for parameter in &descriptor.parameters {
        if parameter.update == UpdateMode::Live {
            let name = scalar_binding(id, &parameter.id)
                .expect("every live parameter has a scalar receiver");
            if !filter.set_parameter(name, number(&parameter.id)) {
                return Err(error(
                    format!("parameters.{}", parameter.id),
                    "filter refused its declared parameter",
                ));
            }
        }
    }
    Ok(RuntimeModule::Filter(filter))
}
