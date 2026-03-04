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
use crate::core::filters::prelude::*;
use crate::core::generator::prelude::builder::*;
use crate::core::generator::prelude::*;
use crate::core::graph::{AudioOutputSink, Filter, Source, System, simple_source};

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
                    let source = create_source(&node_type)
                        .map_err(AppError::AudioError)?;
                    let idx = gs.system.add_source(source);
                    gs.source_map.insert(id, idx);
                }
                NodeKind::Filter => {
                    log::info!("Adding filter ({node_type}) to graph system");
                    let filter = create_filter(&node_type, sample_rate)
                        .map_err(AppError::AudioError)?;
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
            rebuild_and_swap(gs, message_tx)
        }

        GraphCommand::RemoveNode { id } => {
            if let Some(idx) = gs.filter_map.remove(&id) {
                gs.system.remove_filter(idx);
            } else if let Some(idx) = gs.source_map.remove(&id) {
                gs.system.remove_source(idx);
            } else if let Some(idx) = gs.sink_map.remove(&id) {
                gs.system.remove_sink(idx);
            }
            rebuild_and_swap(gs, message_tx)
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
                log::warn!(
                    "Unhandled connection: is_source?={from_is_source} to_is_sink?={to_is_sink}"
                );
            }
            rebuild_and_swap(gs, message_tx)
        }

        GraphCommand::Disconnect { from, to } => {
            if let (Some(&from_idx), Some(&to_idx)) =
                (gs.filter_map.get(&from), gs.filter_map.get(&to))
            {
                let _ = gs.system.disconnect(from_idx, to_idx);
            }
            rebuild_and_swap(gs, message_tx)
        }

        GraphCommand::SetParameter {
            node_id,
            param_name,
            value,
        } => {
            if let Some(&idx) = gs.filter_map.get(&node_id) {
                message_tx
                    .send(AudioMessage::Graph(GraphAudioMessage::SetParameter {
                        node_index: idx.index(),
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
                message_tx
                    .send(AudioMessage::Graph(GraphAudioMessage::StartSource {
                        source_index: idx,
                    }))
                    .map_err(|_| AppError::ChannelClosed)
            } else {
                Ok(())
            }
        }

        GraphCommand::StopNode { id } => {
            if let Some(&idx) = gs.source_map.get(&id) {
                message_tx
                    .send(AudioMessage::Graph(GraphAudioMessage::StopSource {
                        source_index: idx,
                    }))
                    .map_err(|_| AppError::ChannelClosed)
            } else {
                Ok(())
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

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

fn create_source(node_type: &str) -> Result<Box<dyn Source>, String> {
    let waveform: Waveform = node_type.into();
    if let Waveform::Err(name) = waveform {
        return Err(format!("Unknown generator type: {name}"));
    }
    let generator: MultiToneGenerator = ToneGeneratorBuilder::new().waveform(waveform).build().into();
    Ok(simple_source(generator))
}

fn create_filter(node_type: &str, sample_rate: f32) -> Result<Box<dyn Filter>, String> {
    match node_type {
        "Low Pass Filter" => Ok(Box::new(LowPassFilter::new(1000.0, sample_rate))),
        "High Pass Filter" => Ok(Box::new(HighPassFilter::new(1000.0, sample_rate))),
        "GainFilter" => Ok(Box::new(GainFilter::new(1.0))),
        "Tremolo" => Ok(Box::new(Tremolo::new(5.0, 0.5, sample_rate))),
        "Bandpass Filter" => Ok(Box::new(BandPass::new(500.0, 1500.0, sample_rate))),
        "Clipper" => Ok(Box::new(Clipper::new(1.0))),
        "Moving Average" => Ok(Box::new(MovingAverage::new(10))),
        "Delay" => Ok(Box::new(DelayFilter::new(sample_rate, 0.5))),
        "Compressor" => Ok(Box::new(Compressor::default())),
        other => Err(format!("Unknown filter type: {}", other)),
    }
}
