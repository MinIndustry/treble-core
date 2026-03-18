//! Visual graph editor state and command handling.
//!
//! [`GraphData`] holds the user-built audio graph (nodes, connections, ID maps).
//! [`handle_graph_command`] mutates it in response to [`GraphCommand`]s and
//! hot-swaps the result into the render thread via a [`AudioMessage::Graph`] swap.

use std::collections::HashMap;

use petgraph::prelude::NodeIndex;

use crate::app::commands::{GraphCommand, NodeKind};
use crate::app::error::AppError;
use crate::audio::{AudioMessage, GraphAudioMessage};
use crate::core::generator::prelude::builder::*;
use crate::core::generator::prelude::*;
use crate::core::graph::{AudioOutputSink, Filter, ModTarget, SimpleSource, Source, System};

/// State for the visual audio graph editor.
#[derive(Default)]
pub(crate) struct GraphData {
    pub system: System,
    pub filter_map: HashMap<u64, NodeIndex<u32>>,
    pub source_map: HashMap<u64, usize>,
    pub sink_map: HashMap<u64, usize>,
}

pub(crate) fn handle_graph_command(
    cmd: GraphCommand,
    gs: &mut GraphData,
    sample_rate: f32,
    message_tx: &crossbeam::channel::Sender<AudioMessage>,
) -> Result<(), AppError> {
    match cmd {
        GraphCommand::AddNode {
            id,
            node_type,
            kind,
            ..
        } => {
            match kind {
                NodeKind::Generator => {
                    log::info!("Adding generator ({node_type}) to graph system");
                    let source =
                        create_source(&node_type, sample_rate).map_err(AppError::AudioError)?;
                    let idx = gs.system.add_source(source);
                    gs.source_map.insert(id, idx);
                }
                NodeKind::Filter => {
                    log::info!("Adding filter ({node_type}) to graph system");
                    let filter =
                        create_filter(&node_type, sample_rate).map_err(AppError::AudioError)?;
                    let idx = gs.system.add_filter(filter);
                    gs.filter_map.insert(id, idx);
                }
                NodeKind::Sink => {
                    log::info!("Adding sink to graph system");
                    let sink = Box::new(AudioOutputSink::new());
                    let idx = gs.system.add_sink(sink);
                    gs.sink_map.insert(id, idx);
                }
            }
            recompute_topology(gs)
        }

        GraphCommand::RemoveNode { id } => {
            if let Some(idx) = gs.filter_map.remove(&id) {
                gs.system.remove_filter(idx);
                // NodeIndex-based map — petgraph handles index invalidation automatically
            } else if let Some(idx) = gs.source_map.remove(&id) {
                gs.system.remove_source(idx);
                // Vec::remove shifts elements; update all stored source indices
                for v in gs.source_map.values_mut() {
                    if *v > idx {
                        *v -= 1;
                    }
                }
            } else if let Some(idx) = gs.sink_map.remove(&id) {
                gs.system.remove_sink(idx);
                // Vec::remove shifts elements; update all stored sink indices
                for v in gs.sink_map.values_mut() {
                    if *v > idx {
                        *v -= 1;
                    }
                }
            }
            recompute_topology(gs)
        }

        GraphCommand::Connect {
            from,
            from_port,
            to,
            to_port,
        } => {
            let from_is_source = gs.source_map.contains_key(&from);
            let to_is_sink = gs.sink_map.contains_key(&to);

            if from_is_source && !to_is_sink {
                let src_idx = gs.source_map[&from];
                let filter_idx = gs.filter_map[&to];
                gs.system.connect_source(src_idx, filter_idx, to_port);
            } else if !from_is_source && to_is_sink {
                let filter_idx = gs.filter_map[&from];
                let sink_idx = gs.sink_map[&to];
                gs.system.connect_sink(filter_idx, sink_idx, from_port);
            } else if !from_is_source && !to_is_sink {
                let from_idx = gs.filter_map[&from];
                let to_idx = gs.filter_map[&to];
                gs.system.connect(from_idx, to_idx, from_port, to_port);
            } else {
                // from_is_source && to_is_sink: direct wire, no filter in between
                let src_idx = gs.source_map[&from];
                let sink_idx = gs.sink_map[&to];
                log::info!(
                    "[graph] Connect: source {from}(idx={src_idx}) → sink {to}(idx={sink_idx}) direct wire"
                );
                gs.system.connect_source_to_sink(src_idx, sink_idx);
            }
            recompute_topology(gs)
        }

        GraphCommand::Disconnect { from, to } => {
            let from_is_source = gs.source_map.contains_key(&from);
            let to_is_sink = gs.sink_map.contains_key(&to);

            if from_is_source && to_is_sink {
                let src_idx = gs.source_map[&from];
                let sink_idx = gs.sink_map[&to];
                gs.system.disconnect_source_from_sink(src_idx, sink_idx);
            } else if from_is_source && !to_is_sink {
                if let (Some(&src_idx), Some(&filter_idx)) =
                    (gs.source_map.get(&from), gs.filter_map.get(&to))
                {
                    gs.system.disconnect_source(src_idx, filter_idx);
                }
            } else if !from_is_source
                && !to_is_sink
                && let (Some(&from_idx), Some(&to_idx)) =
                    (gs.filter_map.get(&from), gs.filter_map.get(&to))
            {
                let _ = gs.system.disconnect(from_idx, to_idx);
            }
            // filter→sink: sinks track a single (from, port) connection; the new connection
            // from connect_sink() will overwrite it on the next Connect, so no explicit
            // disconnect method is needed.
            recompute_topology(gs)
        }

        GraphCommand::Modulate {
            from,
            to,
            param_name,
        } => {
            if let Some(&src_idx) = gs.source_map.get(&from) {
                let target = if let Some(&filter_idx) = gs.filter_map.get(&to) {
                    Some(ModTarget::Filter(filter_idx))
                } else if let Some(&tgt_src_idx) = gs.source_map.get(&to) {
                    Some(ModTarget::Source(tgt_src_idx))
                } else {
                    None
                };
                if let Some(target) = target {
                    gs.system
                        .add_mod_wire(src_idx, target.clone(), param_name.clone());
                    message_tx
                        .send(AudioMessage::Graph(GraphAudioMessage::AddModulation {
                            from_source: src_idx,
                            target,
                            param_name,
                        }))
                        .map_err(|_| AppError::ChannelClosed)?;
                }
            }
            Ok(())
        }

        GraphCommand::Demodulate {
            from,
            to,
            param_name,
        } => {
            if let Some(&src_idx) = gs.source_map.get(&from) {
                let target = if let Some(&filter_idx) = gs.filter_map.get(&to) {
                    Some(ModTarget::Filter(filter_idx))
                } else if let Some(&tgt_src_idx) = gs.source_map.get(&to) {
                    Some(ModTarget::Source(tgt_src_idx))
                } else {
                    None
                };
                if let Some(target) = target {
                    gs.system.remove_mod_wire(src_idx, &target, &param_name);
                    message_tx
                        .send(AudioMessage::Graph(GraphAudioMessage::RemoveModulation {
                            from_source: src_idx,
                            target,
                            param_name,
                        }))
                        .map_err(|_| AppError::ChannelClosed)?;
                }
            }
            Ok(())
        }

        GraphCommand::SetParameter {
            node_id,
            param_name,
            value,
        } => {
            if let Some(&idx) = gs.filter_map.get(&node_id) {
                // Keep the command-thread topology copy in sync for mix_mode
                if param_name == "mix_mode" {
                    let mode = rustic_meta::MixMode::from_ordinal(value as usize);
                    gs.system.set_mix_mode(idx, mode);
                }
                message_tx
                    .send(AudioMessage::Graph(GraphAudioMessage::SetParameter {
                        node_index: idx.index(),
                        param_name,
                        value,
                    }))
                    .map_err(|_| AppError::ChannelClosed)
            } else if let Some(&src_idx) = gs.source_map.get(&node_id) {
                // Keep the command-thread copy in sync
                gs.system.set_source_parameter(src_idx, &param_name, value);
                message_tx
                    .send(AudioMessage::Graph(GraphAudioMessage::SetSourceParameter {
                        source_index: src_idx,
                        param_name,
                        value,
                    }))
                    .map_err(|_| AppError::ChannelClosed)
            } else {
                Ok(())
            }
        }

        GraphCommand::StartNode { id } => {
            if let Some(&idx) = gs.source_map.get(&id) {
                log::info!(
                    "[graph] StartNode id={id} → source_index={idx}, sending StartSource to render thread"
                );
                message_tx
                    .send(AudioMessage::Graph(GraphAudioMessage::StartSource {
                        source_index: idx,
                    }))
                    .map_err(|_| AppError::ChannelClosed)
            } else {
                log::warn!(
                    "[graph] StartNode id={id} → NOT found in source_map (known sources: {:?})",
                    gs.source_map.keys().collect::<Vec<_>>()
                );
                Ok(())
            }
        }

        GraphCommand::StopNode { id } => {
            if let Some(&idx) = gs.source_map.get(&id) {
                log::info!("[graph] StopNode id={id} → source_index={idx} (graceful stop)");
                message_tx
                    .send(AudioMessage::Graph(GraphAudioMessage::StopSource {
                        source_index: idx,
                    }))
                    .map_err(|_| AppError::ChannelClosed)
            } else {
                Ok(())
            }
        }

        GraphCommand::KillNode { id } => {
            if let Some(&idx) = gs.source_map.get(&id) {
                log::info!("[graph] KillNode id={id} → source_index={idx} (immediate kill)");
                message_tx
                    .send(AudioMessage::Graph(GraphAudioMessage::KillSource {
                        source_index: idx,
                    }))
                    .map_err(|_| AppError::ChannelClosed)
            } else {
                Ok(())
            }
        }

        GraphCommand::Compile => rebuild_and_swap(gs, message_tx),
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Recompile the topology graph in the command-thread copy only.
/// Does NOT push a Swap to the render thread — use `rebuild_and_swap` for that.
fn recompute_topology(gs: &mut GraphData) -> Result<(), AppError> {
    gs.system
        .compute()
        .map_err(|e| AppError::AudioError(format!("{:?}", e)))
}

/// Recompile the topology and immediately hot-swap it into the render thread.
/// Only called by `GraphCommand::Compile`.
fn rebuild_and_swap(
    gs: &mut GraphData,
    message_tx: &crossbeam::channel::Sender<AudioMessage>,
) -> Result<(), AppError> {
    gs.system
        .compute()
        .map_err(|e| AppError::AudioError(format!("{:?}", e)))?;
    let cloned = gs.system.clone();
    message_tx
        .send(AudioMessage::Graph(GraphAudioMessage::Swap(cloned)))
        .map_err(|_| AppError::ChannelClosed)
}

fn create_source(node_type: &str, sample_rate: f32) -> Result<Box<dyn Source>, String> {
    let waveform: Waveform = node_type.into();
    if let Waveform::Err(name) = waveform {
        return Err(format!("Unknown generator type: {name}"));
    }
    // FrequencyRelation::Identity makes update_frequency() track the base frequency in
    // real-time (1:1 ratio), so changing the "frequency" parameter while a note plays
    // immediately affects the pitch. Without a relation the call is a no-op.
    let generator: MultiToneGenerator = ToneGeneratorBuilder::new()
        .waveform(waveform)
        .frequency_relation(FrequencyRelation::Identity)
        .build()
        .into();
    Ok(SimpleSource::new(generator, sample_rate).boxed())
}

fn create_filter(node_type: &str, sample_rate: f32) -> Result<Box<dyn Filter>, String> {
    for entry in inventory::iter::<crate::meta::FilterRegistration>() {
        let info = (entry.info)();
        if info.type_id == node_type {
            let mut filter = (entry.create)();
            // Apply sample_rate to any filter that exposes it as a parameter.
            // Filters without a "sample_rate" field will silently ignore this (logged at debug).
            filter.set_parameter("sample_rate", sample_rate);
            return Ok(filter);
        }
    }
    Err(format!("Unknown filter type: {}", node_type))
}
