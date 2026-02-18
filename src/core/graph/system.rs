use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use petgraph::Graph;
use petgraph::dot::Dot;
use petgraph::prelude::NodeIndex;
use petgraph::{Direction, algo::toposort};

use super::{Filter, Sink, Source};
use crate::core::graph::error::AudioGraphError;

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
#[derive(Debug, Default, Clone)]
#[allow(clippy::type_complexity)]
pub struct System {
    // The actual filter graph, from which the execution order is derived
    // Each weight represents the port into which the filter is connected
    graph: Graph<Box<dyn Filter>, (usize, usize)>,
    // Each layer represents filters that can be run concurrently.
    layers: Vec<Vec<usize>>,
    // The sources of the system and the filters they are connected to
    // The node index is the index of the filter that the source is connected to
    // The second usize is the port of the filter that the source is connected to
    sources: Vec<(Box<dyn Source>, (NodeIndex<u32>, usize))>,
    // The sinks of the system.
    // The node index is the index of the filter that the sink is connected to
    // The second usize is the port of the filter that the sink is connected to
    sinks: Vec<((NodeIndex<u32>, usize), Box<dyn Sink>)>,
}

impl System {
    /// Creates a new system with simple null sources & simple sinks
    #[allow(clippy::type_complexity)]
    pub fn new() -> Self {
        System {
            graph: Graph::new(),
            layers: Vec::new(),
            sources: Vec::new(),
            sinks: Vec::new(),
        }
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
            let (graph_b_source_descendant_index, graph_b_node_port) = other.sources[*to].1;
            let source_descendant =
                dyn_clone::clone_box(&*other.graph[graph_b_source_descendant_index]);

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

            let node = dyn_clone::clone_box(&*other.graph[node_index]);
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
        };

        Ok(new_system)
    }

    // Adds a filter to the system. Further references to this filter should be done using the returned uuid
    pub fn add_filter(&mut self, filter: Box<dyn Filter>) -> NodeIndex<u32> {
        log::trace!("[Graph] Adding filter {:?}", filter);
        self.graph.add_node(filter)
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

    /// Connects a source to a filter of the graph
    pub fn connect_source(&mut self, source: usize, to: NodeIndex<u32>, in_port: usize) {
        self.sources[source].1 = (to, in_port);
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

    /// Sets source at index `index` to be the given source object
    pub fn set_source(
        &mut self,
        index: usize,
        source: Box<dyn Source>,
    ) -> Result<(), AudioGraphError> {
        if index < self.sources.len() {
            log::trace!("[Graph] Setting Node {:?} as source", source);
            self.sources[index] = (source, (NodeIndex::new(0), 0));
            Ok(())
        } else {
            Err(AudioGraphError::InvalidNode)
        }
    }

    /// Adds a source and returns its index
    pub fn add_source(&mut self, source: Box<dyn Source>) -> usize {
        let idx = self.sources.len();
        self.sources.push((source, (NodeIndex::new(0), 0)));
        idx
    }

    /// Removes a source by index
    pub fn remove_source(&mut self, index: usize) -> Option<Box<dyn Source>> {
        if index < self.sources.len() {
            Some(self.sources.remove(index).0)
        } else {
            None
        }
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
            Some(self.sinks.remove(index).1)
        } else {
            None
        }
    }

    /// Removes a filter from the graph
    pub fn remove_filter(&mut self, index: NodeIndex<u32>) -> Option<Box<dyn Filter>> {
        self.graph.remove_node(index)
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

        if let Ok(topo) = toposort(&acyclic_graph, None) {
            for node in topo {
                // TODO: Add same-layer ability (to run some filters in parallel)
                // For each node in the topological order, push the node's index into the layers vector
                // if the next node has no dependencies to the current node, push it to the current layer as well
                // else push it to the next layer
                self.layers.push(vec![node.index()])
            }

            Ok(())
        } else {
            Err(AudioGraphError::CycleDetected)
        }
    }

    // Performs one full run of the system, running every filter once in an order such that data that entered the system this
    // run, can exit it this run as well.
    pub fn run(&mut self) {
        self.sources.iter_mut().for_each(|(source, (desc, port))| {
            let value = source.pull();
            let filter = &mut self.graph[*desc];
            filter.push(value, *port);
        });

        for layer in self.layers.iter() {
            // TODO: Make this parallel
            layer.iter().for_each(|f| {
                let from_node_index = NodeIndex::new(*f);
                let outputs = {
                    let filter = &mut self.graph[from_node_index];
                    filter.transform()
                };

                let neighbours: Vec<NodeIndex> = {
                    self.graph
                        .neighbors_directed(from_node_index, Direction::Outgoing)
                        .collect()
                };

                for neighbour in neighbours {
                    let edges: Vec<(usize, usize)> = {
                        self.graph
                            .edges_connecting(from_node_index, neighbour)
                            .map(|e| *e.weight())
                            .collect()
                    };
                    let neighbour_node = &mut self.graph[neighbour];

                    edges.iter().for_each(|edge| {
                        neighbour_node.push(outputs[edge.0], edge.1);
                    });
                }
            });
        }

        // Makes the sinks pull from their connected nodes
        self.sinks.iter_mut().for_each(|((node, port), sink)| {
            let values = {
                let node = &mut self.graph[*node];
                node.transform()
            };
            sink.push(values[*port], 0);
        });
    }

    /// Starts a source by index (note-on)
    pub fn start_source(&mut self, index: usize) {
        if let Some((source, _)) = self.sources.get_mut(index) {
            source.start();
        }
    }

    /// Stops a source by index (note-off)
    pub fn stop_source(&mut self, index: usize) {
        if let Some((source, _)) = self.sources.get_mut(index) {
            source.stop();
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
        self.graph.node_weight_mut(index)
    }

    pub fn save_to_file(&self, path: &Path) -> Result<(), String> {
        let mut output = File::create(path).map_err(|e| e.to_string())?;
        write!(output, "{:?}", Dot::with_config(&self.graph, &[])).map_err(|e| e.to_string())
    }

    /// Creates a deep clone of this system for handoff to the render thread.
    /// Sources and sinks are cloned via DynClone.
    pub fn clone_for_render(&self) -> System {
        // Implementation depends on Source/Sink also implementing DynClone
        // If they don't, rebuild from scratch using the same topology
        todo!()
    }
}
