use super::{ModuleRegistry, RuntimeModule, error};
use crate::core::graph::{AudioOutputSink, System};
use petgraph::{
    algo::toposort,
    graph::{DiGraph, NodeIndex},
};
use std::collections::BTreeMap;
use treble_meta::{InstrumentDocument, SignalType, ValidationErrors, VoiceConfiguration};

#[derive(Clone, Copy, Debug)]
pub(crate) enum Binding {
    Source(usize),
    Filter(NodeIndex),
}

/// A fully validated, compiled graph. Prepare away from audio processing.
/// Installing replaces DSP state: phase, held notes and effect tails reset.
#[derive(Clone, Debug)]
pub struct PreparedPatch {
    pub(crate) system: System,
    pub(crate) bindings: BTreeMap<String, Binding>,
    pub(crate) document: InstrumentDocument,
    pub(crate) sample_rate: f32,
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
        document.validate_against(self.descriptors())?;
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
            if !matches!(port.signal, SignalType::Audio { channels: 2 }) {
                return Err(error(
                    format!("instrument.patch.cables[{i}]"),
                    "only stereo audio cables are currently supported; use the external note input for triggering",
                ));
            }
            topology.add_edge(
                indices[cable.from.module.as_str()],
                indices[cable.to.module.as_str()],
                (),
            );
        }
        if toposort(&topology, None).is_err() {
            return Err(error(
                "instrument.patch.cables",
                "feedback cycle is unsupported; remove a cable to break the cycle",
            ));
        }
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
            };
            bindings.insert(instance.id.clone(), binding);
        }
        for cable in &document.instrument.patch.cables {
            // All built-in audio sockets have implementation index zero. IDs
            // were validated above; no UI ordering or caller index is trusted.
            match (bindings[&cable.from.module], bindings[&cable.to.module]) {
                (Binding::Source(source), Binding::Filter(filter)) => {
                    system.connect_source(source, filter, 0)
                }
                (Binding::Filter(from), Binding::Filter(to)) => system.connect(from, to, 0, 0),
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
            }
        }
        system
            .compute()
            .map_err(|e| error("instrument.patch", e.to_string()))?;
        if matches!(document.instrument.voice, VoiceConfiguration::Continuous) {
            for binding in bindings.values() {
                if let Binding::Source(index) = *binding {
                    system.start_source(index);
                }
            }
        }
        Ok(PreparedPatch {
            system,
            bindings,
            document: document.clone(),
            sample_rate,
        })
    }
}
