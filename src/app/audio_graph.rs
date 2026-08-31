//! AudioGraph — assembles all instruments into a single compiled `System`.
//!
//! Instrument specs remain serializable data until `compile()` builds their
//! runtime systems. Legacy native instruments are supported as an explicit
//! compatibility path while the old per-sample instrument API is retired.

use std::collections::HashMap;

use petgraph::prelude::NodeIndex;

use crate::core::graph::{
    AudioGraphError, AudioOutputSink, ParameterAutomation, RampCurve, SimpleSink, System,
};
use crate::instruments::Instrument;
use crate::instruments::spec::{
    FxSpec, InstrumentSpec, SpecError, compile_spec_with_fx_nodes, create_filter, validate_spec,
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

/// Which filter a sweep targets, named the way the graph description names it.
///
/// `NodeIndex`es are valid for one compiled `System` only, so a stored
/// automation cannot hold one: it names a declarative position and every
/// `compile()` resolves it against the graph it just built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutomationTarget {
    /// The filter at `fx_index` in instrument slot `slot`'s own `spec.fx` chain.
    InstrumentFx { slot: usize, fx_index: usize },
    /// The filter at `fx_index` in the named bus's shared chain.
    BusFx { bus: String, fx_index: usize },
}

/// The shape of a parameter sweep, independent of what it targets.
///
/// Frames are absolute engine frames, so a sweep that outlives a graph rebuild
/// is picked up by the replacement filter where its predecessor was.
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterRamp {
    pub param: String,
    pub from: f32,
    pub to: f32,
    pub start_frame: u64,
    pub end_frame: u64,
    pub curve: RampCurve,
}

/// A parameter sweep declared against the graph description.
#[derive(Debug, Clone, PartialEq)]
pub struct AutomationSpec {
    pub target: AutomationTarget,
    pub ramp: ParameterRamp,
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
    /// Maps (slot index, index into that slot's spec fx chain) → filter node in
    /// the most recent compiled System. Legacy slots contribute no entries.
    pub instrument_fx_map: HashMap<(usize, usize), NodeIndex<u32>>,
    /// Maps (bus name, index into that bus's chain) → filter node in the most
    /// recent compiled System. Buses that claimed no member are absent.
    pub bus_fx_map: HashMap<(String, usize), NodeIndex<u32>>,
    /// Shared bus chains applied at the next `compile()`.
    buses: Vec<BusSpec>,
    /// Parameter sweeps re-resolved by every `compile()`.
    automations: Vec<AutomationSpec>,
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

    /// The bus set applied at the next `compile()`.
    pub fn buses(&self) -> &[BusSpec] {
        &self.buses
    }

    /// Replace the parameter sweeps applied at the next `compile()`.
    ///
    /// A sweep whose target does not exist in the compiled graph is dropped,
    /// the same way an out-of-range bus member is.
    pub fn set_automations(&mut self, automations: Vec<AutomationSpec>) {
        self.automations = automations;
    }

    /// Append one parameter sweep.
    pub fn add_automation(&mut self, automation: AutomationSpec) {
        self.automations.push(automation);
    }

    /// Drop every declared parameter sweep.
    pub fn clear_automations(&mut self) {
        self.automations.clear();
    }

    /// The declared parameter sweeps, in declaration order.
    pub fn automations(&self) -> &[AutomationSpec] {
        &self.automations
    }

    /// Resolve a declared sweep against the most recent `compile()`, or `None`
    /// when the target names a filter that graph does not contain.
    pub fn resolve_automation(&self, spec: &AutomationSpec) -> Option<ParameterAutomation> {
        let node = match &spec.target {
            AutomationTarget::InstrumentFx { slot, fx_index } => {
                self.instrument_fx_map.get(&(*slot, *fx_index))
            }
            AutomationTarget::BusFx { bus, fx_index } => {
                self.bus_fx_map.get(&(bus.clone(), *fx_index))
            }
        }?;
        Some(ParameterAutomation {
            node: *node,
            param: spec.ramp.param.clone(),
            from: spec.ramp.from,
            to: spec.ramp.to,
            start_frame: spec.ramp.start_frame,
            end_frame: spec.ramp.end_frame,
            curve: spec.ramp.curve,
        })
    }

    pub fn compile(&mut self, sample_rate: f32) -> Result<System, AudioGraphCompileError> {
        if self.instruments.is_empty() {
            return Ok(System::silent());
        }

        let mut main = System::new();
        let n = self.instruments.len();
        let mut output_nodes = Vec::with_capacity(n);

        self.source_map.clear();
        self.instrument_fx_map.clear();
        self.bus_fx_map.clear();

        for (slot_idx, slot) in self.instruments.iter().enumerate() {
            let source_start = main.sources_len();

            let (inst_system, fx_nodes) = match &slot.definition {
                InstrumentDefinition::Spec(spec) => compile_spec_with_fx_nodes(spec, sample_rate)?,
                InstrumentDefinition::Legacy(instrument) => {
                    (instrument.as_system(sample_rate), Vec::new())
                }
            };
            let (output_node, remap) = main.absorb_mapped(inst_system)?;
            for (fx_idx, node) in fx_nodes.iter().enumerate() {
                if let Some(&absorbed) = remap.get(node) {
                    self.instrument_fx_map.insert((slot_idx, fx_idx), absorbed);
                }
            }

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
            for (fx_idx, fx) in bus.fx.iter().enumerate() {
                let mut filter = create_filter(&fx.type_id, sample_rate)?;
                for (param, value) in fx.params.iter() {
                    if crate::instruments::spec::ENGINE_OWNED_PARAMS.contains(&param.as_str()) {
                        continue;
                    }
                    if !filter.set_parameter(param, *value) {
                        return Err(AudioGraphCompileError::Spec(SpecError::UnknownParameter {
                            filter: fx.type_id.clone(),
                            param: param.clone(),
                        }));
                    }
                }
                let filter_index = main.add_filter(filter);
                self.bus_fx_map
                    .insert((bus.name.clone(), fx_idx), filter_index);
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

        // Sweeps are resolved against the graph that was just built, so a
        // rebuild at a loop boundary hands each one to the replacement filter.
        let resolved = self
            .automations
            .iter()
            .filter_map(|spec| self.resolve_automation(spec))
            .collect();
        main.set_automations(resolved);

        Ok(main)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::utils::Note;
    use crate::instruments::registry::InstrumentRegistry;
    use std::collections::HashMap as Map;

    const BLOCK: usize = 512;

    /// The render thread syncs master_volume to sink 0 every block, and it
    /// must degrade rather than die on a system with no sink yet. These two
    /// pin both halves of that contract (the TRBC-RT-001 regression).
    #[test]
    fn a_silent_system_refuses_master_volume_with_a_coded_error() {
        let error = System::silent()
            .set_sink_parameter(0, "master_volume", 0.5)
            .expect_err("a silent system has no sink");
        assert_eq!(error.code(), "TRBC-GRAPH-101");
        assert!(error.to_string().contains("TRBC-GRAPH-101"), "{error}");
    }

    #[test]
    fn the_compiled_graphs_sink_zero_accepts_master_volume() {
        let registry = InstrumentRegistry::built_in();
        let mut graph = AudioGraph::new();
        graph
            .add_spec(registry.get("kick").expect("built-in").clone())
            .expect("kick compiles");
        let mut system = graph.compile(44_100.0).expect("graph compiles");
        assert!(system.set_sink_parameter(0, "master_volume", 0.5).is_ok());
    }

    /// A gain stage, the cheapest filter whose parameter is audible.
    fn gain(factor: f32) -> FxSpec {
        FxSpec {
            type_id: "GainFilter".into(),
            params: Map::from([("factor".to_string(), factor)]),
        }
    }

    /// A one-kick graph with the given buses and sweeps, compiled and struck.
    /// The kick's own fx chain is `extra_fx`, so a test can address slot fx by
    /// index. Returns the graph (for its maps) and the compiled system.
    fn struck_kick(
        extra_fx: Vec<FxSpec>,
        buses: Vec<BusSpec>,
        automations: Vec<AutomationSpec>,
    ) -> (AudioGraph, System) {
        let registry = InstrumentRegistry::built_in();
        let mut spec = registry.get("kick").expect("built-in").clone();
        spec.fx.extend(extra_fx);
        let mut graph = AudioGraph::new();
        let slot = graph.add_spec(spec).expect("kick compiles");
        graph.set_buses(buses);
        graph.set_automations(automations);
        let mut system = graph.compile(44_100.0).expect("graph compiles");
        system.start_note(graph.source_map[&slot], Note::from_midi(36), 1.0);
        (graph, system)
    }

    /// Render `blocks` blocks, evaluating sweeps at each block start the way
    /// the render thread does, and accumulate the master energy.
    fn swept_master_energy(system: &mut System, blocks: u64) -> f32 {
        let mut energy = 0.0;
        for block in 0..blocks {
            system.apply_automations(block * BLOCK as u64);
            system.run_frames(BLOCK);
            energy += system
                .get_sink(0)
                .expect("master sink")
                .consume()
                .iter()
                .map(|frame| frame[0].abs() + frame[1].abs())
                .sum::<f32>();
        }
        energy
    }

    /// A sweep from `from` to `to` over the first `frames` frames.
    fn ramp(from: f32, to: f32, frames: u64) -> ParameterRamp {
        ParameterRamp {
            param: "factor".into(),
            from,
            to,
            start_frame: 0,
            end_frame: frames,
            curve: RampCurve::Linear,
        }
    }

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

    fn drum_bus() -> BusSpec {
        BusSpec {
            name: "drums".into(),
            fx: vec![gain(1.0)],
            members: vec![0],
        }
    }

    #[test]
    fn a_sweep_on_a_bus_filter_changes_the_audible_output() {
        let (_, mut open) = struck_kick(Vec::new(), vec![drum_bus()], Vec::new());
        let full = swept_master_energy(&mut open, 8);

        // Closing over one block leaves only the first block at full gain, so
        // the sweep has to have driven the bus filter for this to differ.
        let closing = AutomationSpec {
            target: AutomationTarget::BusFx {
                bus: "drums".into(),
                fx_index: 0,
            },
            ramp: ramp(1.0, 0.0, BLOCK as u64),
        };
        let (_, mut swept) = struck_kick(Vec::new(), vec![drum_bus()], vec![closing]);
        let faded = swept_master_energy(&mut swept, 8);

        // The kick's noise component reseeds per build, so energies from
        // separate builds are compared for audibility and ordering, never for
        // equality.
        assert!(full > 0.01, "the kick must be audible: {full}");
        assert!(
            faded > 0.001,
            "the first block passes at full gain: {faded}"
        );
        assert!(
            faded < full * 0.8,
            "the sweep did not close: {faded}/{full}"
        );
    }

    #[test]
    fn a_sweep_on_an_instrument_fx_addresses_the_spec_chain() {
        // The appended gain sits after whatever the instrument already carries,
        // which is the whole point of the index: naming a raw chain position
        // would sweep the kick's own filter instead. Read the offset from the
        // spec rather than hard-coding it, so shaping an instrument's voice
        // cannot silently re-point this sweep.
        let own_fx = InstrumentRegistry::built_in()
            .get("kick")
            .expect("built-in")
            .fx
            .len();
        let closing = AutomationSpec {
            target: AutomationTarget::InstrumentFx {
                slot: 0,
                fx_index: own_fx,
            },
            ramp: ramp(1.0, 0.0, BLOCK as u64),
        };
        let (graph, mut swept) = struck_kick(vec![gain(1.0)], Vec::new(), vec![closing]);
        assert!(graph.instrument_fx_map.contains_key(&(0, own_fx)));
        assert_eq!(swept.automations().len(), 1);

        let faded = swept_master_energy(&mut swept, 8);
        let (_, mut open) = struck_kick(vec![gain(1.0)], Vec::new(), Vec::new());
        let full = swept_master_energy(&mut open, 8);
        assert!(
            faded < full * 0.8,
            "the sweep did not close: {faded}/{full}"
        );
    }

    #[test]
    fn the_sweep_set_survives_a_recompile() {
        let sweep = AutomationSpec {
            target: AutomationTarget::BusFx {
                bus: "drums".into(),
                fx_index: 0,
            },
            ramp: ramp(1.0, 0.0, 4_096),
        };
        let registry = InstrumentRegistry::built_in();
        let mut graph = AudioGraph::new();
        graph
            .add_spec(registry.get("kick").expect("built-in").clone())
            .expect("kick compiles");
        graph.set_buses(vec![drum_bus()]);
        graph.set_automations(vec![sweep.clone()]);

        let first = graph.compile(44_100.0).expect("graph compiles");
        let second = graph.compile(44_100.0).expect("graph recompiles");
        assert_eq!(graph.automations(), [sweep]);
        assert_eq!(first.automations().len(), 1);
        assert_eq!(second.automations().len(), 1);
        // Absolute engine frames, not offsets: the replacement filter has to
        // pick the ramp up where its predecessor was.
        assert_eq!(second.automations()[0].start_frame, 0);
        assert_eq!(second.automations()[0].end_frame, 4_096);
    }

    #[test]
    fn a_sweep_naming_no_compiled_filter_is_dropped() {
        let stray = AutomationSpec {
            target: AutomationTarget::BusFx {
                bus: "ghost".into(),
                fx_index: 3,
            },
            ramp: ramp(1.0, 0.0, 4_096),
        };
        let (_, mut system) = struck_kick(Vec::new(), vec![drum_bus()], vec![stray]);
        assert!(system.automations().is_empty());
        assert!(swept_master_energy(&mut system, 8) > 0.01);
    }

    #[test]
    fn a_refused_parameter_is_reported_once() {
        let bogus = AutomationSpec {
            target: AutomationTarget::BusFx {
                bus: "drums".into(),
                fx_index: 0,
            },
            ramp: ParameterRamp {
                param: "cutoff".into(),
                ..ramp(1.0, 0.0, 4_096)
            },
        };
        let (_, mut system) = struck_kick(Vec::new(), vec![drum_bus()], vec![bogus]);
        for block in 0..8 {
            system.apply_automations(block * BLOCK as u64);
            system.run_frames(BLOCK);
        }
        assert_eq!(system.take_automation_warnings().len(), 1);
        system.apply_automations(0);
        assert!(system.take_automation_warnings().is_empty());
    }
}
