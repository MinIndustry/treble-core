use super::{ModuleRegistry, RuntimeModule, error};
use crate::core::graph::{
    AudioOutputSink, ExecutionNode, ParameterModulation, SamplePlan, ScheduledNode, SignalRoute,
    System,
};
use petgraph::{
    algo::toposort,
    graph::{DiGraph, NodeIndex},
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use treble_meta::{
    ControlDomain, ControlRate, InstrumentDocument, PortBinding, SignalType, ValidationErrors,
    VoiceConfiguration,
};

#[derive(Clone, Copy, Debug)]
pub(crate) enum Binding {
    Source(usize),
    Filter(NodeIndex),
    NoteInput,
}

/// A fully validated, compiled graph. Prepare away from audio processing.
/// Installing replaces DSP state: phase, held notes and effect tails reset.
#[derive(Clone, Debug)]
pub struct PreparedPatch {
    pub(crate) system: System,
    pub(crate) bindings: BTreeMap<String, Binding>,
    pub(crate) document: InstrumentDocument,
    pub(crate) sample_rate: f32,
    pub(crate) note_routes: BTreeMap<String, Vec<usize>>,
}

impl PreparedPatch {
    /// Resolve a stable source ID for a host using the lower-level `System` API.
    pub fn source_index(&self, module: &str) -> Option<usize> {
        match self.bindings.get(module) {
            Some(Binding::Source(index)) => Some(*index),
            _ => None,
        }
    }
    pub fn document(&self) -> &InstrumentDocument {
        &self.document
    }
    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }
    /// Use a compiled patch in an existing host-owned graph renderer.
    pub fn into_system(self) -> System {
        self.system
    }
}

impl ModuleRegistry {
    /// Validate runtime capabilities and build an existing `System`. This does
    /// not change a running graph. Disconnected instruments are valid and silent.
    pub fn prepare(
        &self,
        document: &InstrumentDocument,
        sample_rate: f32,
        block_size: usize,
    ) -> Result<PreparedPatch, ValidationErrors> {
        document.validate()?;
        let mut representable = document.clone();
        for module in &mut representable.instrument.patch.modules {
            for value in module.parameters.values_mut() {
                *value = super::modules::runtime_value(value);
            }
        }
        representable.validate_against(self.descriptors())?;
        if !sample_rate.is_finite() || !(8000.0..=192000.0).contains(&sample_rate) {
            return Err(error("sample_rate", "expected 8000..=192000 Hz"));
        }
        if block_size == 0 || block_size > 65536 {
            return Err(error("block_size", "expected 1..=65536 frames"));
        }
        if matches!(
            document.instrument.voice,
            VoiceConfiguration::Polyphonic { .. }
        ) {
            return Err(error(
                "instrument.voice",
                "rack runtime supports continuous or monophonic instruments; polyphony is not implemented",
            ));
        }
        // Check every edge, including disconnected components. Delay nodes do
        // not implicitly make arbitrary feedback contracts valid.
        let mut topology = DiGraph::<(), ()>::new();
        let indices: BTreeMap<_, _> = document
            .instrument
            .patch
            .modules
            .iter()
            .map(|m| (m.id.as_str(), topology.add_node(())))
            .collect();
        for (i, cable) in document.instrument.patch.cables.iter().enumerate() {
            let instance = document
                .instrument
                .patch
                .modules
                .iter()
                .find(|m| m.id == cable.from.module)
                .expect("validated endpoint");
            let descriptor = self
                .descriptor(&instance.type_id, instance.type_version)
                .expect("validated module");
            let port = descriptor
                .ports
                .iter()
                .find(|p| p.id == cable.from.port)
                .expect("validated port");
            if !matches!(
                port.signal,
                SignalType::Audio { channels: 2 }
                    | SignalType::Note
                    | SignalType::Control {
                        domain: ControlDomain::Bipolar,
                        rate: ControlRate::Sample
                    }
            ) {
                return Err(error(
                    format!("instrument.patch.cables[{i}]"),
                    "only stereo audio, notes and bipolar sample control cables are supported",
                ));
            }
            topology.add_edge(
                indices[cable.from.module.as_str()],
                indices[cable.to.module.as_str()],
                (),
            );
        }
        let order = toposort(&topology, None).map_err(|_| {
            let components = petgraph::algo::kosaraju_scc(&topology);
            let cycle = components.iter().find(|component| component.len() > 1 || topology.contains_edge(component[0], component[0])).expect("toposort found a cycle");
            let (index, cable) = document.instrument.patch.cables.iter().enumerate().find(|(_, cable)| cycle.contains(&indices[cable.from.module.as_str()]) && cycle.contains(&indices[cable.to.module.as_str()])).expect("cycle has a cable");
            error(format!("instrument.patch.cables[{index}]"), format!("audio/modulation feedback cycle through cable `{}`; remove this cable to break the cycle", cable.id))
        })?;
        let mut system = System::new().with_block_size(block_size);
        let mut bindings = BTreeMap::new();
        for (i, instance) in document.instrument.patch.modules.iter().enumerate() {
            let runtime = self
                .instantiate(instance, sample_rate)
                .map_err(|mut errors| {
                    for e in &mut errors.0 {
                        e.path = format!("instrument.patch.modules[{i}].{}", e.path);
                    }
                    errors
                })?;
            let binding = match runtime {
                RuntimeModule::Source(source) => Binding::Source(system.add_source(source)),
                RuntimeModule::Filter(filter) => Binding::Filter(system.add_filter(filter)),
                RuntimeModule::NoteInput => Binding::NoteInput,
            };
            bindings.insert(instance.id.clone(), binding);
        }
        for cable in &document.instrument.patch.cables {
            let module = document
                .instrument
                .patch
                .modules
                .iter()
                .find(|m| m.id == cable.from.module)
                .expect("validated module");
            let descriptor = self
                .descriptor(&module.type_id, module.type_version)
                .expect("validated descriptor");
            let output = descriptor
                .ports
                .iter()
                .find(|p| p.id == cable.from.port)
                .expect("validated port");
            if matches!(output.signal, SignalType::Control { .. }) {
                continue;
            }
            // All built-in audio sockets have implementation index zero. IDs
            // were validated above; no UI ordering or caller index is trusted.
            match (bindings[&cable.from.module], bindings[&cable.to.module]) {
                (Binding::Source(source), Binding::Filter(filter)) => {
                    system.connect_source(source, filter, 0)
                }
                (Binding::Filter(from), Binding::Filter(to)) => system.connect(from, to, 0, 0),
                (Binding::NoteInput, Binding::Source(_) | Binding::NoteInput) => {}
                _ => {
                    return Err(error(
                        "instrument.patch.cables",
                        "unsupported audio endpoint binding",
                    ));
                }
            }
        }
        let sink = system.add_sink(Box::new(AudioOutputSink::new(sample_rate)));
        for output in &document.instrument.interface.audio_outputs {
            match bindings[&output.module] {
                Binding::Source(source) => system.connect_source_to_sink(source, sink),
                Binding::Filter(filter) => system.connect_sink(filter, sink, 0),
                Binding::NoteInput => {
                    return Err(error(
                        "instrument.interface.audio_outputs",
                        "note input cannot supply audio",
                    ));
                }
            }
        }
        system
            .compute()
            .map_err(|e| error("instrument.patch", e.to_string()))?;
        let module_by_node: BTreeMap<_, _> =
            indices.iter().map(|(id, node)| (*node, *id)).collect();
        let ordered: Vec<_> = order
            .into_iter()
            .filter_map(|node| {
                let id = module_by_node[&node];
                match bindings[id] {
                    Binding::Source(index) => Some((id, ExecutionNode::Source(index))),
                    Binding::Filter(index) => Some((id, ExecutionNode::Filter(index))),
                    Binding::NoteInput => None,
                }
            })
            .collect();
        let positions: BTreeMap<_, _> = ordered
            .iter()
            .enumerate()
            .map(|(index, (id, _))| (*id, index))
            .collect();
        let mut nodes: Vec<_> = ordered
            .iter()
            .map(|(_, node)| ScheduledNode {
                node: *node,
                parameters: Vec::new(),
                routes: Vec::new(),
            })
            .collect();
        let empty = Arc::new(Vec::new());
        let mut cables: Vec<_> = document.instrument.patch.cables.iter().collect();
        cables.sort_by(|a, b| a.id.cmp(&b.id));
        let mut has_modulation = false;
        for cable in cables {
            let target = document
                .instrument
                .patch
                .modules
                .iter()
                .find(|m| m.id == cable.to.module)
                .expect("validated target");
            let descriptor = self
                .descriptor(&target.type_id, target.type_version)
                .expect("validated descriptor");
            let port = descriptor
                .ports
                .iter()
                .find(|p| p.id == cable.to.port)
                .expect("validated input");
            let PortBinding::Parameter { parameter_id } = &port.binding else {
                continue;
            };
            let parameter = descriptor
                .parameters
                .iter()
                .find(|p| p.id == *parameter_id)
                .expect("validated parameter");
            let modulation = parameter.modulation.as_ref().expect("validated modulation");
            let name =
                super::session::scalar_binding(&target.type_id, parameter_id).ok_or_else(|| {
                    error(
                        format!("modules.{}.parameters.{parameter_id}", target.id),
                        "sample modulation receiver is unsupported",
                    )
                })?;
            let mode = cable.modulation.as_ref().map_or_else(
                || modulation.default_mode.clone(),
                |setting| setting.mode.clone(),
            );
            let depth = cable
                .modulation
                .as_ref()
                .map_or(modulation.default_depth, |setting| setting.depth);
            let scheduled = &mut nodes[positions[cable.to.module.as_str()]];
            let group = if let Some(index) = scheduled
                .parameters
                .iter()
                .position(|group| group.parameter == name)
            {
                index
            } else {
                scheduled.parameters.push(ParameterModulation {
                    parameter: name.into(),
                    mode,
                    mapping: modulation.mapping.clone(),
                    domain: parameter.domain.clone(),
                    signals: Vec::new(),
                });
                scheduled.parameters.len() - 1
            };
            let contribution = scheduled.parameters[group].signals.len();
            scheduled.parameters[group]
                .signals
                .push((Arc::clone(&empty), depth));
            scheduled.routes.push(SignalRoute {
                source: positions[cable.from.module.as_str()],
                port: 0,
                group,
                contribution,
            });
            has_modulation = true;
        }
        if has_modulation {
            let outputs = vec![Vec::new(); nodes.len()];
            system
                .install_sample_plan(SamplePlan {
                    valid: true,
                    nodes,
                    outputs,
                })
                .map_err(|e| error("instrument.patch", e.to_string()))?;
        }
        if matches!(document.instrument.voice, VoiceConfiguration::Continuous) {
            for binding in bindings.values() {
                if let Binding::Source(index) = *binding {
                    system.start_source(index);
                }
            }
        }
        let mut note_routes = BTreeMap::new();
        for (id, binding) in &bindings {
            let mut sources = Vec::new();
            let mut pending = vec![id.as_str()];
            let mut visited = BTreeSet::new();
            while let Some(id) = pending.pop() {
                if !visited.insert(id) {
                    continue;
                }
                match bindings[id] {
                    Binding::Source(index) => sources.push(index),
                    Binding::NoteInput => {
                        pending.extend(
                            document
                                .instrument
                                .patch
                                .cables
                                .iter()
                                .filter(|c| c.from.module == id)
                                .map(|c| c.to.module.as_str()),
                        );
                    }
                    Binding::Filter(_) => {}
                }
            }
            sources.sort_unstable();
            sources.dedup();
            let instance = document
                .instrument
                .patch
                .modules
                .iter()
                .find(|m| m.id == *id)
                .expect("validated module");
            let descriptor = self
                .descriptor(&instance.type_id, instance.type_version)
                .expect("validated descriptor");
            if matches!(binding, Binding::Source(_) | Binding::NoteInput)
                && descriptor.ports.iter().any(|port| {
                    port.direction == treble_meta::Direction::Input
                        && port.signal == SignalType::Note
                })
            {
                note_routes.insert(id.clone(), sources);
            }
        }
        Ok(PreparedPatch {
            system,
            bindings,
            document: document.clone(),
            sample_rate,
            note_routes,
        })
    }
}
