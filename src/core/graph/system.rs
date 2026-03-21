use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use petgraph::Graph;
use petgraph::dot::Dot;
use petgraph::prelude::NodeIndex;
use petgraph::{Direction, algo::toposort};
use rustic_meta::MixMode;

use super::audio_node::AudioNode;
use super::{Filter, Sink, Source};
use crate::core::audio::Block;
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
/// use rustic::core::graph::System;
/// use rustic::core::filters::prelude::Tremolo;
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
    // The sources of the system and the filters they are connected to.
    // Each source may fan out to multiple (filter, port) pairs.
    sources: Vec<(Box<dyn Source>, Vec<(NodeIndex<u32>, usize)>)>,
    // The sinks of the system.
    // The node index is the index of the filter that the sink is connected to
    // The second usize is the port of the filter that the sink is connected to
    sinks: Vec<((NodeIndex<u32>, usize), Box<dyn Sink>)>,
    /// Direct source→sink wires that bypass the filter graph entirely.
    /// Each entry is (source_index, sink_index).
    source_sink_wires: Vec<(usize, usize)>,
    /// Live modulation wires: a source's block-mean drives a named parameter.
    mod_wires: Vec<ModWire>,
    /// Number of frames to produce per `run()` call
    block_size: usize,
}

impl Default for System {
    fn default() -> Self {
        System {
            graph: Graph::new(),
            layers: Vec::new(),
            sources: Vec::new(),
            sinks: Vec::new(),
            source_sink_wires: Vec::new(),
            mod_wires: Vec::new(),
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

    /// Merges the two systems to create a new one. The graphs are merged following the given mapping from sinks to sources.
    /// Sinks to sources links are replaced with a simple combinator filter. The amount of input in the second system
    /// should match the amount of output in the first system.
    #[allow(clippy::type_complexity)]
    pub fn merge(
        mut self,
        other: System,
        mapping: Vec<(usize, usize)>,
    ) -> Result<System, AudioGraphError> {
        if self.sinks.len() != other.sinks.len() {
            log::error!("Trying to merge graphs with incompatible shapes");
            return Err(AudioGraphError::InvalidMerging);
        }

        // Contains the mapping other graph -> new graph
        let mut new_edge_map: HashMap<NodeIndex, NodeIndex> = HashMap::new();

        log::trace!("Merging two graphs together");
        for (from, to) in mapping.iter() {
            let (graph_b_source_descendant_index, graph_b_node_port) = other.sources[*to]
                .1
                .first()
                .copied()
                .unwrap_or((NodeIndex::new(0), 0));
            let source_descendant = other.graph[graph_b_source_descendant_index].clone();

            // Save the new index of the source descendant
            let new_index = if let std::collections::hash_map::Entry::Vacant(e) =
                new_edge_map.entry(graph_b_source_descendant_index)
            {
                let new_index = self.graph.add_node(source_descendant);
                log::info!(
                    "idx {} -> idx {}",
                    graph_b_source_descendant_index.index(),
                    new_index.index()
                );
                e.insert(new_index);
                new_index
            } else {
                match new_edge_map.get(&graph_b_source_descendant_index) {
                    Some(v) => *v,
                    None => panic!("What ?"),
                }
            };

            // Connect the sink's predecessors to the source's successors
            let (sink_predecessor_id, sink_predecessor_port) = self.sinks[*from].0;
            log::trace!(
                "\tNode {} -> Sink {} & Source {} -> Node {} => Node {} -> Node {}",
                sink_predecessor_id.index(),
                from,
                to,
                graph_b_source_descendant_index.index(),
                sink_predecessor_id.index(),
                new_index.index()
            );
            self.graph.add_edge(
                sink_predecessor_id,
                new_index,
                (sink_predecessor_port, graph_b_node_port),
            );
        }

        // Go through all nodes in the other graph and add them to the new graph
        for node_index in other.graph.node_indices() {
            // Skip already added nodes
            if new_edge_map.contains_key(&node_index) {
                continue;
            }

            let node = other.graph[node_index].clone();
            let new_index = self.graph.add_node(node);
            new_edge_map.insert(node_index, new_index);
        }

        // Now that all nodes are added, connect the edges
        for edge in other.graph.edge_indices() {
            let (other_from, other_to) = other.graph.edge_endpoints(edge).unwrap();
            let (from, to) = (new_edge_map[&other_from], new_edge_map[&other_to]);
            log::info!(
                "\tEdge ({}, {}) -> ({}, {})",
                other_from.index(),
                other_to.index(),
                from.index(),
                to.index()
            );
            let weight = other.graph[edge];
            self.graph.add_edge(from, to, weight);
        }

        let new_sinks: Vec<_> = other
            .sinks
            .iter()
            .map(|sink| {
                (
                    (new_edge_map[&sink.0.0], sink.0.1),
                    dyn_clone::clone_box(&*sink.1),
                )
            })
            .collect();

        let new_system: System = System {
            graph: self.graph,
            layers: self.layers,
            sources: self.sources,
            sinks: new_sinks,
            source_sink_wires: Vec::new(),
            mod_wires: Vec::new(),
            block_size: self.block_size,
        };

        Ok(new_system)
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

    /// Connects a filter from the graph to a sink
    pub fn connect_sink(&mut self, from: NodeIndex<u32>, sink: usize, out_port: usize) {
        log::info!("Node {} (p: {}) -> Sink {}", from.index(), out_port, sink);
        self.sinks[sink].0 = (from, out_port);
    }

    /// Sets the sink at index `index` to be the given sink object
    pub fn set_sink(&mut self, index: usize, sink: Box<dyn Sink>) -> Result<(), AudioGraphError> {
        if index < self.sinks.len() {
            log::trace!("[Graph] Setting Node {:?} as sink {}", sink, index);
            self.sinks[index] = ((NodeIndex::new(0), 0), sink);
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
    pub fn set_source_parameter(&mut self, index: usize, name: &str, value: f32) {
        if let Some((source, _)) = self.sources.get_mut(index) {
            source.set_parameter(name, value);
        }
    }

    /// Returns the number of sinks currently registered in this system.
    pub fn sinks_len(&self) -> usize {
        self.sinks.len()
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
    pub fn add_mod_wire(&mut self, from_source: usize, target: ModTarget, param_name: String) {
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
        self.sinks.push(((NodeIndex::new(0), 0), sink));
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
        for node in topo {
            // TODO: Add same-layer ability (to run some filters in parallel)
            self.layers.push(vec![node.index()])
        }

        Ok(())
    }

    // Performs one full run of the system, running every filter once in an order such that data
    // that entered the system this run can exit it this run as well.
    pub fn run(&mut self) {
        let block_size = self.block_size;

        // Pull from all sources; push directly to connected AudioNodes (fan-out).
        // Two-step collect releases the borrow on self.sources before we touch self.graph.
        let source_blocks: Vec<Arc<Block>> = self
            .sources
            .iter_mut()
            .enumerate()
            .map(|(i, (source, connections))| {
                let block = source.pull(block_size);
                log::trace!("[system::run] source[{i}] active={}", source.is_active());
                (block, connections)
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|(block, connections)| {
                let block = Arc::from(block);
                for (desc, port) in connections {
                    if let Some(node) = self.graph.node_weight_mut(*desc) {
                        node.push(Arc::clone(&block), *port);
                    }
                }
                block
            })
            .collect();

        // Apply live modulation: drive target parameters from source block means.
        let mod_actions: Vec<(ModTarget, String, f32)> = self
            .mod_wires
            .iter()
            .filter_map(|wire| {
                let block = source_blocks.get(wire.from_source)?;
                let mean = if block.is_empty() {
                    0.0
                } else {
                    block.iter().map(|f| (f[0] + f[1]) * 0.5).sum::<f32>() / block.len() as f32
                };
                Some((wire.target.clone(), wire.param_name.clone(), mean))
            })
            .collect();
        for (target, param_name, value) in mod_actions {
            match target {
                ModTarget::Source(idx) => {
                    if let Some((src, _)) = self.sources.get_mut(idx) {
                        src.set_parameter(&param_name, value);
                    }
                }
                ModTarget::Filter(node_idx) => {
                    if let Some(node) = self.graph.node_weight_mut(node_idx) {
                        node.filter_mut().set_parameter(&param_name, value);
                    }
                }
            }
        }

        // Process filters layer by layer.
        for layer in self.layers.iter() {
            for &f in layer.iter() {
                let node_idx = NodeIndex::new(f);

                // Process: mix accumulated inputs, run transform, get Arc-wrapped outputs.
                let outputs = self.graph[node_idx].process(block_size);

                // Push outputs to downstream nodes.
                let neighbours: Vec<NodeIndex> = self
                    .graph
                    .neighbors_directed(node_idx, Direction::Outgoing)
                    .collect();
                for neighbour in neighbours {
                    let edges: Vec<(usize, usize)> = self
                        .graph
                        .edges_connecting(node_idx, neighbour)
                        .map(|e| *e.weight())
                        .collect();
                    for (out_port, in_port) in edges {
                        if let Some(block) = outputs.get(out_port)
                            && let Some(node) = self.graph.node_weight_mut(neighbour)
                        {
                            node.push(Arc::clone(block), in_port);
                        }
                    }
                }

                // Push outputs to connected sinks inline (no second pass needed).
                for ((sink_node, sink_port), sink) in &mut self.sinks {
                    if *sink_node == node_idx
                        && let Some(block) = outputs.get(*sink_port)
                    {
                        log::trace!(
                            "[system::run] sink ← NodeIndex({}) port={sink_port}",
                            node_idx.index()
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
            }
        }
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
        if other.sinks.is_empty() {
            return Err(AudioGraphError::InvalidMerging);
        }

        // Record which filter node fed other's first sink before we consume other
        let (other_output_node, _) = other.sinks[0].0;

        // Import all filter nodes, building old-index → new-index mapping
        let mut remap: HashMap<NodeIndex<u32>, NodeIndex<u32>> = HashMap::new();
        for old_idx in other.graph.node_indices() {
            let node = other.graph[old_idx].clone();
            let new_idx = self.graph.add_node(node);
            remap.insert(old_idx, new_idx);
        }

        // Re-add edges with remapped endpoints
        for edge_idx in other.graph.edge_indices() {
            let (from, to) = other.graph.edge_endpoints(edge_idx).unwrap();
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
        remap
            .get(&other_output_node)
            .copied()
            .ok_or(AudioGraphError::InvalidNode)
    }
}
