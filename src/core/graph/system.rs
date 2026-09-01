use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use petgraph::Graph;
use petgraph::dot::Dot;
use petgraph::prelude::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::{Direction, algo::toposort};
use treble_meta::MixMode;

use super::audio_node::AudioNode;
use super::automation::ParameterAutomation;
use super::{Filter, Sink, Source};
use crate::core::graph::error::AudioGraphError;

/// Target of a modulation wire.
#[derive(Debug, Clone, PartialEq)]
pub enum ModTarget {
    /// A source node (by source-Vec index).
    Source(usize),
    /// A filter node (by petgraph NodeIndex).
    Filter(NodeIndex<u32>),
}

/// A live modulation connection: source block-mean drives a named parameter.
#[derive(Debug, Clone)]
pub struct ModWire {
    /// Index into `System::sources` — the modulating oscillator.
    pub from_source: usize,
    /// What gets modulated.
    pub target: ModTarget,
    /// The parameter name forwarded to `set_parameter`.
    pub param_name: String,
}

/// Work performed by the most recent [`System::run_frames`] call.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RunStats {
    pub frames: usize,
    pub active_sources: usize,
    pub processed_nodes: usize,
    pub source_routes: usize,
    pub filter_routes: usize,
    pub sink_routes: usize,
}

/// ## A Pipe & Filter system
/// The system is composed of filters, sources and sinks.
/// It is represented as a directed graph where the filters are the nodes.
/// The Sources & Sinks are special nodes that have respectively only outgoing (source) or incoming (sink) edges.
/// The edges represent the pipes between the filters.
/// Some filters have special output properties. E.g. the delay filter's input pipe is ignored (postponed) when
/// the topology sorting is done, in order to avoid cycles. A system with cycles must include a delay or similar filter
/// to break the cycle.
///
/// ```rust
/// use treble::core::graph::System;
/// use treble::core::filters::prelude::Tremolo;
///
/// // A simple system with one input and one output
/// let mut system = System::new();
///
/// // Adding a filter to the system
/// let filter = Tremolo::new(20.0, 0.5, 44100.0);
/// let filter_index = system.add_filter(Box::from(filter));
/// ```
#[derive(Debug, Clone)]
#[allow(clippy::type_complexity)]
pub struct System {
    // The actual filter graph, from which the execution order is derived
    // Each weight represents the port into which the filter is connected
    graph: Graph<AudioNode, (usize, usize)>,
    // Each layer represents filters that can be run concurrently.
    layers: Vec<Vec<usize>>,
    /// Precomputed filter and sink routes, indexed by the source node index.
    /// Rebuilt by `compute()` so graph traversal and temporary route vectors
    /// stay off the audio thread.
    compiled: Box<CompiledState>,
    // The sources of the system and the filters they are connected to.
    // Each source may fan out to multiple (filter, port) pairs.
    sources: Vec<(Box<dyn Source>, Vec<(NodeIndex<u32>, usize)>)>,
    // The sinks of the system.
    // Each sink may receive output from multiple filter nodes (sources).
    // Each source is identified by (NodeIndex, output_port).
    sinks: Vec<(Vec<(NodeIndex<u32>, usize)>, Box<dyn Sink>)>,
    /// Direct source→sink wires that bypass the filter graph entirely.
    /// Each entry is (source_index, sink_index).
    source_sink_wires: Vec<(usize, usize)>,
    /// Live modulation wires: a source's block-mean drives a named parameter.
    mod_wires: Vec<ModWire>,
    /// Timed parameter sweeps — see [`System::apply_automations`].
    automation: Box<AutomationState>,
    /// Number of frames to produce per `run()` call
    block_size: usize,
}

#[derive(Debug, Clone, Copy)]
struct FilterRoute {
    target: NodeIndex<u32>,
    output_port: usize,
    input_port: usize,
}

#[derive(Debug, Clone, Copy)]
struct SinkRoute {
    sink: usize,
    output_port: usize,
}

#[derive(Debug, Clone, Default)]
struct NodeDispatch {
    filters: Vec<FilterRoute>,
    sinks: Vec<SinkRoute>,
}

#[derive(Debug, Clone, Default)]
struct CompiledState {
    dispatch: Vec<NodeDispatch>,
    last_run_stats: RunStats,
}

/// Sweep bookkeeping, boxed: a whole `System` travels by value inside the
/// render-thread message enums, so keeping it out of the struct body keeps
/// those messages from growing a hundred bytes for state used once a block.
#[derive(Debug, Clone, Default)]
struct AutomationState {
    automations: Vec<ParameterAutomation>,
    /// (node, parameter) pairs already reported as refused. A sweep is applied
    /// every block, so without this a single bad parameter name would emit a
    /// backend event per block for as long as the graph lives.
    reported: HashSet<(NodeIndex<u32>, String)>,
    /// Rejection messages not yet drained by the render thread.
    pending: Vec<String>,
}

impl Default for System {
    fn default() -> Self {
        System {
            graph: Graph::new(),
            layers: Vec::new(),
            compiled: Box::default(),
            sources: Vec::new(),
            sinks: Vec::new(),
            source_sink_wires: Vec::new(),
            mod_wires: Vec::new(),
            automation: Box::default(),
            block_size: 512,
        }
    }
}

impl System {
    /// Creates a new system with a default block size of 512 frames.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the mix strategy for a filter node. Changes take effect on the next `run()` call.
    pub fn set_mix_mode(&mut self, node: NodeIndex<u32>, mode: MixMode) {
        if let Some(n) = self.graph.node_weight_mut(node) {
            n.set_mix_mode(mode);
        }
    }

    /// Returns the mix strategy for a filter node, defaulting to `Sum`.
    pub fn get_mix_mode(&self, node: NodeIndex<u32>) -> MixMode {
        self.graph
            .node_weight(node)
            .map(|n| n.mix_mode())
            .unwrap_or_default()
    }

    /// Builder-style setter for the block size.
    pub fn with_block_size(mut self, n: usize) -> Self {
        self.block_size = n;
        self
    }

    /// Returns the current block size.
    pub fn block_size(&self) -> usize {
        self.block_size
    }

    /// Work counters from the most recent graph run.
    pub fn last_run_stats(&self) -> RunStats {
        self.compiled.last_run_stats
    }

    // Adds a filter to the system. Further references to this filter should be done using the returned uuid
    pub fn add_filter(&mut self, filter: Box<dyn Filter>) -> NodeIndex<u32> {
        log::trace!("[Graph] Adding filter {:?}", filter);
        self.graph
            .add_node(AudioNode::new(filter, MixMode::default()))
    }

    // Connects two filters together. This method connects the filter in the topology graph as well.
    // Do not use this function to close a feedback loop. Use the connect_feedback method instead.
    pub fn connect(
        &mut self,
        from: NodeIndex<u32>,
        to: NodeIndex<u32>,
        out_port: usize,
        in_port: usize,
    ) {
        log::trace!(
            "[Graph] Connecting {:?} (p: {}) to {:?} (p: {})",
            self.graph[from],
            out_port,
            self.graph[to],
            in_port
        );
        self.graph.add_edge(from, to, (out_port, in_port));
    }

    /// Connects a source directly to a sink, bypassing any filters.
    pub fn connect_source_to_sink(&mut self, source_idx: usize, sink_idx: usize) {
        if !self.source_sink_wires.contains(&(source_idx, sink_idx)) {
            self.source_sink_wires.push((source_idx, sink_idx));
        }
    }

    /// Removes a direct source→sink wire.
    pub fn disconnect_source_from_sink(&mut self, source_idx: usize, sink_idx: usize) {
        self.source_sink_wires
            .retain(|&(s, k)| s != source_idx || k != sink_idx);
    }

    /// Connects a source to a filter of the graph (fan-out: one source can feed many filters).
    pub fn connect_source(&mut self, source: usize, to: NodeIndex<u32>, in_port: usize) {
        self.sources[source].1.push((to, in_port));
    }

    /// Removes the connection from a source to a specific filter.
    pub fn disconnect_source(&mut self, source: usize, filter: NodeIndex<u32>) {
        if let Some((_, connections)) = self.sources.get_mut(source) {
            connections.retain(|&(n, _)| n != filter);
        }
    }

    /// Connects a filter node's output to a sink.
    /// Multiple calls add multiple source connections to the same sink.
    pub fn connect_sink(&mut self, from: NodeIndex<u32>, sink: usize, out_port: usize) {
        log::info!("Node {} (p: {}) -> Sink {}", from.index(), out_port, sink);
        self.sinks[sink].0.push((from, out_port));
    }

    /// Sets the sink at index `index` to be the given sink object (preserves existing sources).
    pub fn set_sink(&mut self, index: usize, sink: Box<dyn Sink>) -> Result<(), AudioGraphError> {
        if index < self.sinks.len() {
            log::trace!("[Graph] Setting Node {:?} as sink {}", sink, index);
            self.sinks[index].1 = sink;
            Ok(())
        } else {
            Err(AudioGraphError::InvalidNode)
        }
    }

    /// Sets source at index `index` to be the given source object (clears existing connections).
    pub fn set_source(
        &mut self,
        index: usize,
        source: Box<dyn Source>,
    ) -> Result<(), AudioGraphError> {
        if index < self.sources.len() {
            log::trace!("[Graph] Setting Node {:?} as source", source);
            self.sources[index] = (source, vec![]);
            Ok(())
        } else {
            Err(AudioGraphError::InvalidNode)
        }
    }

    /// Returns the number of sources currently registered in this system.
    pub fn sources_len(&self) -> usize {
        self.sources.len()
    }

    /// Set a named parameter on a source by index.
    pub fn set_source_parameter(
        &mut self,
        index: usize,
        name: &str,
        value: f32,
    ) -> Result<(), AudioGraphError> {
        let (source, _) = self
            .sources
            .get_mut(index)
            .ok_or(AudioGraphError::InvalidNode)?;
        source
            .set_parameter(name, value)
            .then_some(())
            .ok_or_else(|| AudioGraphError::UnknownParameter {
                target: format!("source {index}"),
                parameter: name.to_owned(),
            })
    }

    /// Returns the number of sinks currently registered in this system.
    pub fn sinks_len(&self) -> usize {
        self.sinks.len()
    }

    /// Set a named parameter on a sink by index.
    pub fn set_sink_parameter(
        &mut self,
        sink_idx: usize,
        name: &str,
        value: f32,
    ) -> Result<(), AudioGraphError> {
        let (_, sink) = self
            .sinks
            .get_mut(sink_idx)
            .ok_or(AudioGraphError::InvalidNode)?;
        sink.set_parameter(name, value)
            .then_some(())
            .ok_or_else(|| AudioGraphError::UnknownParameter {
                target: format!("sink {sink_idx}"),
                parameter: name.to_owned(),
            })
    }

    /// Returns the number of computed layers (0 means graph not yet compiled).
    pub fn layers_len(&self) -> usize {
        self.layers.len()
    }

    /// Adds a source and returns its index
    pub fn add_source(&mut self, source: Box<dyn Source>) -> usize {
        let idx = self.sources.len();
        self.sources.push((source, vec![]));
        idx
    }

    /// Removes a source by index
    pub fn remove_source(&mut self, index: usize) -> Option<Box<dyn Source>> {
        if index < self.sources.len() {
            let removed = self.sources.remove(index).0;
            // Drop direct source→sink wires; shift remaining source indices
            self.source_sink_wires.retain(|&(s, _)| s != index);
            for (s, _) in self.source_sink_wires.iter_mut() {
                if *s > index {
                    *s -= 1;
                }
            }
            // Drop mod wires involving the removed source; reindex surviving entries
            self.mod_wires.retain(|w| w.from_source != index);
            for w in self.mod_wires.iter_mut() {
                if w.from_source > index {
                    w.from_source -= 1;
                }
                if let ModTarget::Source(ref mut t) = w.target
                    && *t > index
                {
                    *t -= 1;
                }
            }
            Some(removed)
        } else {
            None
        }
    }

    /// Register a live modulation wire: the block mean of `from_source` will
    /// drive `param_name` on `target` every `run()` call.
    pub fn add_mod_wire(
        &mut self,
        from_source: usize,
        target: ModTarget,
        param_name: String,
    ) -> Result<(), AudioGraphError> {
        if from_source >= self.sources.len() {
            return Err(AudioGraphError::InvalidNode);
        }
        let supports_parameter = match target {
            ModTarget::Source(index) => self
                .sources
                .get(index)
                .is_some_and(|(source, _)| source.supports_parameter(&param_name)),
            ModTarget::Filter(index) => self
                .graph
                .node_weight(index)
                .is_some_and(|node| node.filter().supports_parameter(&param_name)),
        };
        if !supports_parameter {
            return Err(AudioGraphError::UnknownParameter {
                target: format!("{target:?}"),
                parameter: param_name,
            });
        }
        // Avoid duplicates
        if !self.mod_wires.iter().any(|w| {
            w.from_source == from_source && w.target == target && w.param_name == param_name
        }) {
            self.mod_wires.push(ModWire {
                from_source,
                target,
                param_name,
            });
        }
        Ok(())
    }

    /// Remove an existing modulation wire.
    pub fn remove_mod_wire(&mut self, from_source: usize, target: &ModTarget, param_name: &str) {
        self.mod_wires.retain(|w| {
            !(w.from_source == from_source && &w.target == target && w.param_name == param_name)
        });
    }

    /// Adds a sink and returns its index
    pub fn add_sink(&mut self, sink: Box<dyn Sink>) -> usize {
        let idx = self.sinks.len();
        self.sinks.push((vec![], sink));
        idx
    }

    /// Removes a sink by index
    pub fn remove_sink(&mut self, index: usize) -> Option<Box<dyn Sink>> {
        if index < self.sinks.len() {
            let removed = self.sinks.remove(index).1;
            // Drop wires involving the removed sink; shift remaining sink indices
            self.source_sink_wires.retain(|&(_, k)| k != index);
            for (_, k) in self.source_sink_wires.iter_mut() {
                if *k > index {
                    *k -= 1;
                }
            }
            Some(removed)
        } else {
            None
        }
    }

    /// Removes a filter from the graph
    pub fn remove_filter(&mut self, index: NodeIndex<u32>) -> Option<Box<dyn Filter>> {
        self.graph.remove_node(index).map(|n| n.filter)
    }

    /// Disconnects two filters
    pub fn disconnect(
        &mut self,
        from: NodeIndex<u32>,
        to: NodeIndex<u32>,
    ) -> Result<(), AudioGraphError> {
        if let Some(edge) = self.graph.find_edge(from, to) {
            self.graph.remove_edge(edge);
            Ok(())
        } else {
            Err(AudioGraphError::ConnectionNotAllowed)
        }
    }

    // Creates the execution layers by sorting the graph topologically.
    #[allow(clippy::result_unit_err)]
    pub fn compute(&mut self) -> Result<(), AudioGraphError> {
        self.layers.clear();
        self.compiled.dispatch.clear();

        // Makes the graph acyclic to be able to create a topology sort
        let acyclic_graph = self.graph.filter_map(
            |_index, node| Some(node),
            |index, edge| {
                if self
                    .graph
                    .edge_endpoints(index)
                    .map(|(_, to)| self.graph[to].postponable())
                    == Some(true)
                {
                    None
                } else {
                    Some(edge)
                }
            },
        );

        let topo = toposort(&acyclic_graph, None).map_err(|_| AudioGraphError::CycleDetected)?;

        // Assign each node its topological depth (longest path from any root).
        // Iterating in topological order guarantees all predecessors are visited
        // before the current node, so their depths are already in the map.
        let mut depth: HashMap<NodeIndex, usize> = HashMap::with_capacity(topo.len());
        for &node in &topo {
            let d = acyclic_graph
                .neighbors_directed(node, Direction::Incoming)
                .map(|pred| depth.get(&pred).copied().unwrap_or(0) + 1)
                .max()
                .unwrap_or(0);
            depth.insert(node, d);
        }

        // Group nodes into layers by depth. All nodes sharing a depth have no
        // intra-layer data dependency and may execute concurrently.
        let max_depth = depth.values().max().copied().unwrap_or(0);
        self.layers = vec![Vec::new(); max_depth + 1];
        for &node in &topo {
            self.layers[depth[&node]].push(node.index());
        }

        // Compile all static graph and sink routing once. Size by the largest
        // node index rather than assuming indices are dense after mutations.
        let dispatch_len = self
            .graph
            .node_indices()
            .map(|node| node.index())
            .max()
            .map_or(0, |max_index| max_index + 1);
        self.compiled
            .dispatch
            .resize_with(dispatch_len, NodeDispatch::default);
        for edge in self.graph.edge_references() {
            let (output_port, input_port) = *edge.weight();
            self.compiled.dispatch[edge.source().index()]
                .filters
                .push(FilterRoute {
                    target: edge.target(),
                    output_port,
                    input_port,
                });
        }
        for (sink, (sources, _)) in self.sinks.iter().enumerate() {
            for &(node, output_port) in sources {
                self.compiled.dispatch[node.index()]
                    .sinks
                    .push(SinkRoute { sink, output_port });
            }
        }

        Ok(())
    }

    /// Tell every filter where the transport is — see [`Filter::on_transport`].
    ///
    /// Called by the render thread once per rendered block, before `run`.
    /// A plain node walk: no allocation, and a no-op for filters that do not
    /// override the hook.
    pub fn broadcast_transport(&mut self, frame: u64) {
        for node in self.graph.node_weights_mut() {
            node.on_transport(frame);
        }
    }

    /// Replace the sweep set. Node indices belong to this compiled graph only,
    /// so callers that survive a rebuild keep their sweeps as declarative data
    /// and resolve them again for every new `System`.
    pub fn set_automations(&mut self, automations: Vec<ParameterAutomation>) {
        self.automation.automations = automations;
        self.automation.reported.clear();
    }

    /// Add one sweep to the set.
    pub fn add_automation(&mut self, automation: ParameterAutomation) {
        self.automation.automations.push(automation);
    }

    /// The active sweeps.
    pub fn automations(&self) -> &[ParameterAutomation] {
        &self.automation.automations
    }

    /// Evaluate every sweep at `frame` and push the values into their filters.
    ///
    /// Called by the render thread once per rendered block, alongside
    /// [`broadcast_transport`](Self::broadcast_transport): both anchor to the
    /// engine timeline so a hot-swap mid-performance continues rather than
    /// restarts.
    pub fn apply_automations(&mut self, frame: u64) {
        // Empty until something is actually refused, so the normal path stays
        // allocation-free.
        let mut refused: Vec<(NodeIndex<u32>, String)> = Vec::new();
        for automation in &self.automation.automations {
            let value = automation.value_at(frame);
            let applied = self
                .graph
                .node_weight_mut(automation.node)
                .is_some_and(|node| node.filter_mut().set_parameter(&automation.param, value));
            if !applied {
                refused.push((automation.node, automation.param.clone()));
            }
        }
        for (node, param) in refused {
            if self.automation.reported.insert((node, param.clone())) {
                self.automation.pending.push(format!(
                    "filter node {} refused automated parameter '{param}'",
                    node.index()
                ));
            }
        }
    }

    /// Drain the sweep rejections reported since the last call, for the render
    /// thread to forward as backend errors. Each (node, parameter) pair is
    /// reported once per graph.
    pub fn take_automation_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.automation.pending)
    }

    // Performs one full run of the system, running every filter once in an order such that data
    // that entered the system this run can exit it this run as well.
    pub fn run(&mut self) {
        self.run_frames(self.block_size);
    }

    /// Performs one run producing `frames` frames instead of the configured
    /// block size. The render thread uses this to split a block at scheduled
    /// note-event boundaries (sample-accurate timing); all DSP nodes are
    /// block-size-agnostic so any `frames >= 1` is valid.
    pub fn run_frames(&mut self, frames: usize) {
        let block_size = frames;
        let mut stats = RunStats {
            frames,
            ..RunStats::default()
        };

        // Pull from all sources; push directly to connected AudioNodes (fan-out).
        // Two-step collect releases the borrow on self.sources before we touch self.graph.
        let mut source_blocks = Vec::with_capacity(self.sources.len());
        for (i, (source, connections)) in self.sources.iter_mut().enumerate() {
            let block = Arc::new(source.pull(block_size));
            log::trace!("[system::run] source[{i}] active={}", source.is_active());
            stats.active_sources += usize::from(source.is_active());
            stats.source_routes += connections.len();
            for &(target, port) in connections.iter() {
                if let Some(node) = self.graph.node_weight_mut(target) {
                    node.push(Arc::clone(&block), port);
                }
            }
            source_blocks.push(block);
        }

        // Apply live modulation: drive target parameters from source block means.
        for wire in &self.mod_wires {
            let Some(block) = source_blocks.get(wire.from_source) else {
                continue;
            };
            let value = if block.is_empty() {
                0.0
            } else {
                block.iter().map(|f| (f[0] + f[1]) * 0.5).sum::<f32>() / block.len() as f32
            };
            match &wire.target {
                ModTarget::Source(idx) => {
                    if let Some((src, _)) = self.sources.get_mut(*idx) {
                        debug_assert!(src.set_parameter(&wire.param_name, value));
                    }
                }
                ModTarget::Filter(node_idx) => {
                    if let Some(node) = self.graph.node_weight_mut(*node_idx) {
                        debug_assert!(node.filter_mut().set_parameter(&wire.param_name, value));
                    }
                }
            }
        }

        // Process filters layer by layer.
        for layer in self.layers.iter() {
            for &f in layer.iter() {
                let node_idx = NodeIndex::new(f);
                let outputs = self.graph[node_idx].process(block_size);
                stats.processed_nodes += 1;
                let Some(dispatch) = self.compiled.dispatch.get(f) else {
                    continue;
                };
                for route in &dispatch.filters {
                    stats.filter_routes += 1;
                    if let Some(block) = outputs.get(route.output_port)
                        && let Some(node) = self.graph.node_weight_mut(route.target)
                    {
                        node.push(Arc::clone(block), route.input_port);
                    }
                }
                for route in &dispatch.sinks {
                    stats.sink_routes += 1;
                    if let Some(block) = outputs.get(route.output_port)
                        && let Some((_, sink)) = self.sinks.get_mut(route.sink)
                    {
                        log::trace!(
                            "[system::run] sink ← NodeIndex({}) port={}",
                            node_idx.index(),
                            route.output_port
                        );
                        sink.push(Arc::clone(block), 0);
                    }
                }
            }
        }

        // Push source blocks to directly-wired sinks.
        for &(src_idx, sink_idx) in &self.source_sink_wires {
            if let Some(block) = source_blocks.get(src_idx)
                && let Some((_, sink)) = self.sinks.get_mut(sink_idx)
            {
                sink.push(Arc::clone(block), 0);
                stats.sink_routes += 1;
            }
        }
        self.compiled.last_run_stats = stats;
    }

    /// Starts a source by index (note-on)
    pub fn start_source(&mut self, index: usize) {
        if let Some((source, _)) = self.sources.get_mut(index) {
            log::info!(
                "[system] start_source({index}): was_active={}",
                source.is_active()
            );
            source.start();
            log::info!(
                "[system] start_source({index}): now_active={}",
                source.is_active()
            );
        } else {
            log::warn!(
                "[system] start_source({index}): index out of range (sources.len={})",
                self.sources.len()
            );
        }
    }

    /// Stops a source by index (note-off, lets release envelope finish).
    pub fn stop_source(&mut self, index: usize) {
        if let Some((source, _)) = self.sources.get_mut(index) {
            source.stop();
        }
    }

    /// Hard-kills a source by index (immediate silence, ignores envelope).
    pub fn kill_source(&mut self, index: usize) {
        if let Some((source, _)) = self.sources.get_mut(index) {
            source.kill();
        }
    }

    /// Returns whether a source is still active (producing audio)
    pub fn is_source_active(&self, index: usize) -> bool {
        self.sources
            .get(index)
            .map(|(s, _)| s.is_active())
            .unwrap_or(false)
    }

    /// Sends a start_note event to a source by index.
    pub fn start_note(&mut self, index: usize, note: crate::core::utils::Note, velocity: f32) {
        if let Some((source, _)) = self.sources.get_mut(index) {
            source.start_note(note, velocity);
        }
    }

    /// Sends a stop_note event to a source by index.
    pub fn stop_note(&mut self, index: usize, note: crate::core::utils::Note) {
        if let Some((source, _)) = self.sources.get_mut(index) {
            source.stop_note(note);
        }
    }

    /// Returns a sink pipe from the system. If the index is out of bounds, returns an error.
    pub fn get_sink(&mut self, index: usize) -> Result<&mut Box<dyn Sink>, &str> {
        self.sinks
            .get_mut(index)
            .map(|s| &mut s.1)
            .ok_or("Index out of bounds")
    }

    /// Returns a mutable reference to a filter in the graph by its NodeIndex.
    /// This allows direct access to filter-specific methods (like reset()) that aren't
    /// part of the Filter trait.
    pub fn get_filter_mut(&mut self, index: NodeIndex<u32>) -> Option<&mut Box<dyn Filter>> {
        self.graph.node_weight_mut(index).map(|n| n.filter_mut())
    }

    /// Saves the graph to a file
    pub fn save_to_file(&self, path: &Path) -> Result<(), String> {
        let mut output = File::create(path).map_err(|e| e.to_string())?;
        write!(output, "{:?}", Dot::with_config(&self.graph, &[])).map_err(|e| e.to_string())
    }

    /// Creates a deep clone of this system for handoff to the render thread.
    pub fn clone_for_render(&self) -> System {
        self.clone()
    }

    /// Returns an empty `System` that produces silence.
    ///
    /// `run()` on a silent system is a no-op; `get_sink(0)` returns `Err`,
    /// so the render thread produces an empty chunk (→ ring buffer silence).
    /// Use this to initialise the render thread before any instruments are loaded.
    pub fn silent() -> Self {
        System::new()
    }

    /// Absorbs all filter nodes, edges, and sources from `other` into `self`,
    /// remapping `NodeIndex`es to the new graph.
    ///
    /// Returns the remapped `NodeIndex` of the filter that was feeding `other`'s
    /// first sink — the "output node" of the absorbed sub-graph — so the caller
    /// can wire it into a master combinator or sink.
    ///
    /// `other`'s sinks are intentionally **not** imported; the caller is responsible
    /// for providing a master sink and connecting to the returned output node.
    pub fn absorb(&mut self, other: System) -> Result<NodeIndex<u32>, AudioGraphError> {
        self.absorb_mapped(other).map(|(output, _)| output)
    }

    /// [`absorb`](Self::absorb), additionally reporting the old → new node
    /// index mapping. Parameter automation addresses a filter by its position
    /// in an instrument spec's chain, which is only known while building the
    /// sub-graph, so the caller needs the remap to keep that address usable.
    #[allow(clippy::type_complexity)]
    pub fn absorb_mapped(
        &mut self,
        other: System,
    ) -> Result<(NodeIndex<u32>, HashMap<NodeIndex<u32>, NodeIndex<u32>>), AudioGraphError> {
        if other.sinks.is_empty() {
            return Err(AudioGraphError::InvalidMerging);
        }

        // Record which filter node fed other's first sink before we consume other
        let &(other_output_node, _) = other.sinks[0]
            .0
            .first()
            .ok_or(AudioGraphError::InvalidMerging)?;

        // Import all filter nodes, building old-index → new-index mapping
        let mut remap: HashMap<NodeIndex<u32>, NodeIndex<u32>> = HashMap::new();
        for old_idx in other.graph.node_indices() {
            let node = other.graph[old_idx].clone();
            let new_idx = self.graph.add_node(node);
            remap.insert(old_idx, new_idx);
        }

        // Re-add edges with remapped endpoints. An edge index from the
        // graph's own iterator always has endpoints; skipping (with a coded
        // log) rather than panicking keeps a hypothetical corruption from
        // killing a live rebuild.
        for edge_idx in other.graph.edge_indices() {
            let Some((from, to)) = other.graph.edge_endpoints(edge_idx) else {
                log::warn!("TRBC-GRAPH-110: absorbed graph edge without endpoints; skipped");
                continue;
            };
            let weight = other.graph[edge_idx];
            self.graph.add_edge(remap[&from], remap[&to], weight);
        }

        // Transfer sources, remapping their connected filter NodeIndexes
        for (source, connections) in other.sources {
            let remapped: Vec<(NodeIndex<u32>, usize)> = connections
                .into_iter()
                .map(|(node_idx, port)| {
                    let remapped_idx = remap.get(&node_idx).copied().unwrap_or(node_idx);
                    (remapped_idx, port)
                })
                .collect();
            self.sources.push((source, remapped));
        }

        // Return the remapped output node so the caller can connect it
        let output = remap
            .get(&other_output_node)
            .copied()
            .ok_or(AudioGraphError::InvalidNode)?;
        Ok((output, remap))
    }
}
