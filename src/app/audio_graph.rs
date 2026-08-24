//! AudioGraph — assembles all instruments into a single compiled `System`.
//!
//! Instrument specs remain serializable data until `compile()` builds their
//! runtime systems. Legacy native instruments are supported as an explicit
//! compatibility path while the old per-sample instrument API is retired.

use std::collections::HashMap;

use crate::core::graph::{AudioGraphError, AudioOutputSink, SimpleSink, System};
use crate::instruments::Instrument;
use crate::instruments::spec::{
    FxSpec, InstrumentSpec, SpecError, compile_spec, create_filter, validate_spec,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AudioGraphCompileError {
    #[error(transparent)]
    Graph(#[from] AudioGraphError),
    #[error(transparent)]
    Spec(#[from] SpecError),
}

/// The persistent definition behind an instrument slot.
pub enum InstrumentDefinition {
    /// Canonical path: serializable data, compiled afresh for every graph build.
    Spec(Box<InstrumentSpec>),
    /// Compatibility path for custom/native instruments that are not specs yet.
    Legacy(Box<dyn Instrument>),
}

/// A single instrument slot inside the audio graph.
pub struct InstrumentSlot {
    pub definition: InstrumentDefinition,
    /// Optional per-instrument filter chain (merged after `into_system()`).
    /// Currently unused — reserved for Phase 4.
    pub filters: Option<System>,
}

/// A shared filter chain: the named members sum into one instance of the
/// chain before the master sink, instead of connecting to it directly. This is
/// a true bus — one reverb tail or compressor for the whole group.
#[derive(Debug, Clone)]
pub struct BusSpec {
    pub name: String,
    /// The shared chain, in order. An empty chain is legal: the bus is then
    /// only a grouping, and members connect straight to the sink.
    pub fx: Vec<FxSpec>,
    /// Instrument slot indices routed through this bus.
    pub members: Vec<usize>,
}

/// Manages all instruments and compiles them into a single `System`.
///
/// After calling `compile()`, `source_map` records which source index inside
/// the compiled `System` belongs to each instrument slot (by slot index).
/// Use those indices with `System::start_note()` / `System::stop_note()`.
#[derive(Default)]
pub struct AudioGraph {
    instruments: Vec<InstrumentSlot>,
    /// Maps slot index → first source index in the most recent compiled System.
    pub source_map: HashMap<usize, usize>,
    /// Shared bus chains applied at the next `compile()`.
    buses: Vec<BusSpec>,
}

impl AudioGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an instrument and return its slot index.
    pub fn add_instrument(&mut self, instrument: Box<dyn Instrument>) -> usize {
        self.push(InstrumentDefinition::Legacy(instrument))
    }

    /// Validate and append a serializable instrument specification.
    pub fn add_spec(&mut self, spec: InstrumentSpec) -> Result<usize, SpecError> {
        validate_spec(&spec)?;
        Ok(self.push(InstrumentDefinition::Spec(Box::new(spec))))
    }

    /// Replace a canonical slot without changing its stable slot index.
    pub fn replace_spec(&mut self, slot_idx: usize, spec: InstrumentSpec) -> Result<(), SpecError> {
        validate_spec(&spec)?;
        let slot = self
            .instruments
            .get_mut(slot_idx)
            .ok_or_else(|| SpecError::Other(format!("unknown instrument slot {slot_idx}")))?;
        slot.definition = InstrumentDefinition::Spec(Box::new(spec));
        Ok(())
    }

    fn push(&mut self, definition: InstrumentDefinition) -> usize {
        let idx = self.instruments.len();
        self.instruments.push(InstrumentSlot {
            definition,
            filters: None,
        });
        idx
    }

    /// Return the retained spec for a canonical slot, or `None` for legacy slots.
    pub fn spec(&self, slot_idx: usize) -> Option<&InstrumentSpec> {
        match &self.instruments.get(slot_idx)?.definition {
            InstrumentDefinition::Spec(spec) => Some(spec),
            InstrumentDefinition::Legacy(_) => None,
        }
    }

    /// Number of instrument slots.
    pub fn len(&self) -> usize {
        self.instruments.len()
    }

    pub fn is_empty(&self) -> bool {
        self.instruments.is_empty()
    }

    /// Compiles all instrument slots into a unified `System`.
    ///
    /// The returned `System` has one `AudioOutputSink` and is ready to be
    /// swapped into the render thread. The `source_map` is updated so the
    /// caller can route `NoteStart`/`NoteStop` to the correct source index.
    /// Replace the bus set used by the next `compile()`.
    ///
    /// A slot claimed by more than one bus is routed by the first bus that
    /// names it; out-of-range members are ignored.
    pub fn set_buses(&mut self, buses: Vec<BusSpec>) {
        self.buses = buses;
    }

    pub fn compile(&mut self, sample_rate: f32) -> Result<System, AudioGraphCompileError> {
        if self.instruments.is_empty() {
            return Ok(System::silent());
        }

        let mut main = System::new();
        let n = self.instruments.len();
        let mut output_nodes = Vec::with_capacity(n);

        self.source_map.clear();

        for (slot_idx, slot) in self.instruments.iter().enumerate() {
            let source_start = main.sources_len();

            let inst_system = match &slot.definition {
                InstrumentDefinition::Spec(spec) => compile_spec(spec, sample_rate)?,
                InstrumentDefinition::Legacy(instrument) => instrument.as_system(sample_rate),
            };
            let output_node = main.absorb(inst_system)?;

            // Every absorbed source up to source_start is this instrument's
            let source_count = main.sources_len() - source_start;
            if source_count > 0 {
                self.source_map.insert(slot_idx, source_start);
            }

            output_nodes.push(output_node);
        }

        // The sink sums all instrument streams, applies master volume and limits.
        let sink = Box::new(AudioOutputSink::new(sample_rate));
        let sink_idx = main.add_sink(sink);

        // Bussed slots run through their shared chain first; the rest connect
        // directly. Filter inputs default to MixMode::Sum, so fanning several
        // members into the chain's first node is the summing itself.
        let mut bussed: Vec<bool> = vec![false; output_nodes.len()];
        for bus in &self.buses {
            let members: Vec<usize> = bus
                .members
                .iter()
                .copied()
                .filter(|&slot| slot < output_nodes.len() && !bussed[slot])
                .collect();
            if members.is_empty() {
                continue;
            }
            for &slot in &members {
                bussed[slot] = true;
            }
            let mut previous = None;
            for fx in &bus.fx {
                let mut filter = create_filter(&fx.type_id, sample_rate)?;
                for (param, value) in fx.params.iter() {
                    if !filter.set_parameter(param, *value) {
                        return Err(AudioGraphCompileError::Spec(SpecError::UnknownParameter {
                            filter: fx.type_id.clone(),
                            param: param.clone(),
                        }));
                    }
                }
                let filter_index = main.add_filter(filter);
                match previous {
                    None => {
                        for &slot in &members {
                            main.connect(output_nodes[slot], filter_index, 0, 0);
                        }
                    }
                    Some(previous_index) => {
                        main.connect(previous_index, filter_index, 0, 0);
                    }
                }
                previous = Some(filter_index);
            }
            match previous {
                // A chain-less bus is only a grouping.
                None => {
                    for &slot in &members {
                        main.connect_sink(output_nodes[slot], sink_idx, 0);
                    }
                }
                Some(last) => main.connect_sink(last, sink_idx, 0),
            }
        }
        for (slot, &out_node) in output_nodes.iter().enumerate() {
            if !bussed[slot] {
                main.connect_sink(out_node, sink_idx, 0);
            }
        }

        // Post-instrument taps preserve isolated slot output for optional
        // multitrack recording. Sink 0 remains the limited master; tap N+1
        // corresponds to application instrument slot N.
        for &out_node in &output_nodes {
            let tap = main.add_sink(Box::new(SimpleSink::new()));
            main.connect_sink(out_node, tap, 0);
        }

        main.compute()?;

        Ok(main)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::utils::Note;
    use crate::instruments::registry::InstrumentRegistry;
    use std::collections::HashMap as Map;

    /// Compile a one-kick graph, strike a note, and return the master energy.
    fn master_energy(buses: Vec<BusSpec>) -> (f32, f32) {
        let registry = InstrumentRegistry::built_in();
        let mut graph = AudioGraph::new();
        let slot = graph
            .add_spec(registry.get("kick").expect("built-in").clone())
            .expect("kick compiles");
        graph.set_buses(buses);
        let mut system = graph.compile(44_100.0).expect("graph compiles");
        let source = graph.source_map[&slot];
        system.start_note(source, Note::from_midi(36), 1.0);
        system.run_frames(4_096);
        let master: f32 = system
            .get_sink(0)
            .expect("master sink")
            .consume()
            .iter()
            .map(|frame| frame[0].abs() + frame[1].abs())
            .sum();
        // Sink 1 is the slot's isolated pre-bus tap.
        let tap: f32 = system
            .get_sink(1)
            .expect("slot tap")
            .consume()
            .iter()
            .map(|frame| frame[0].abs() + frame[1].abs())
            .sum();
        (master, tap)
    }

    #[test]
    fn a_bus_chain_processes_the_members_it_claims() {
        let (unbussed, tap_a) = master_energy(Vec::new());
        assert!(unbussed > 0.01, "the kick must be audible: {unbussed}");

        // A gain-zero bus silences the master while the pre-bus tap still
        // carries the isolated slot signal for multitrack recording.
        let mute = BusSpec {
            name: "drums".into(),
            fx: vec![FxSpec {
                type_id: "GainFilter".into(),
                params: Map::from([("factor".to_string(), 0.0f32)]),
            }],
            members: vec![0],
        };
        let (bussed, tap_b) = master_energy(vec![mute]);
        assert!(bussed < 1e-6, "the bus chain was bypassed: {bussed}");
        // The kick's noise component reseeds per build, so tap energies are
        // compared for audibility rather than equality.
        assert!(tap_b > 0.01, "taps must stay pre-bus: {tap_b}");
        assert!(tap_a > 0.01);
    }

    #[test]
    fn an_empty_bus_is_only_a_grouping() {
        let (unbussed, _) = master_energy(Vec::new());
        let grouping = BusSpec {
            name: "drums".into(),
            fx: Vec::new(),
            members: vec![0],
        };
        let (bussed, _) = master_energy(vec![grouping]);
        assert!(unbussed > 0.01 && bussed > 0.01, "{unbussed} vs {bussed}");
    }

    #[test]
    fn out_of_range_members_are_ignored() {
        let stray = BusSpec {
            name: "ghost".into(),
            fx: Vec::new(),
            members: vec![7],
        };
        let (master, _) = master_energy(vec![stray]);
        assert!(master > 0.01, "an invalid bus must not eat the signal");
    }
}
