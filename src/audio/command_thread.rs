//! Command processing thread implementation

use crate::app::commands::AudioCommand;
use crate::app::prelude::*;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{self, JoinHandle};

use super::RenderMode;
use super::events::BackendEvent;
use super::messages::{AudioMessage, GraphAudioMessage};
use super::shared_state::SharedAudioState;

use crate::app::commands::{GraphCommand, NodeKind};
use crate::core::filters::prelude::*;
use crate::core::generator::prelude::builder::*;
use crate::core::generator::prelude::*;
use crate::core::graph::{AudioOutputSink, Filter, Source, System, simple_source};
use petgraph::prelude::NodeIndex;
use std::collections::HashMap;

/// Holds the graph data for the current run
#[derive(Default)]
struct GraphData {
    system: System,
    filter_map: HashMap<u64, NodeIndex<u32>>,
    source_map: HashMap<u64, usize>,
    sink_map: HashMap<u64, usize>,
}

fn create_source(node_type: &str, params: &[(String, f32)]) -> Result<Box<dyn Source>, String> {
    let freq = params
        .iter()
        .find(|(name, _)| name == "frequency")
        .map(|(_, v)| *v)
        .unwrap_or(440.0);

    let waveform = match node_type {
        "Sine Wave" => Waveform::Sine,
        "Square Wave" => Waveform::Square,
        "Sawtooth Wave" => Waveform::Sawtooth,
        "Triangle Wave" => Waveform::Triangle,
        "White Noise" => Waveform::WhiteNoise,
        other => return Err(format!("Unknown generator type: {}", other)),
    };

    let generator: MultiToneGenerator = ToneGeneratorBuilder::new()
        .waveform(waveform)
        .frequency(freq)
        .build()
        .into();

    Ok(simple_source(generator))
}

fn create_filter(
    node_type: &str,
    params: &[(String, f32)],
    sample_rate: f32,
) -> Result<Box<dyn Filter>, String> {
    let get_param = |name: &str, default: f32| -> f32 {
        params
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| *v)
            .unwrap_or(default)
    };

    match node_type {
        "Low Pass Filter" => Ok(Box::new(LowPassFilter::new(
            get_param("cutoff_frequency", 1000.0),
            sample_rate,
        ))),
        "High Pass Filter" => Ok(Box::new(HighPassFilter::new(
            get_param("cutoff_frequency", 1000.0),
            sample_rate,
        ))),
        "GainFilter" => Ok(Box::new(GainFilter::new(get_param("factor", 1.0)))),
        "Tremolo" => Ok(Box::new(Tremolo::new(
            get_param("frequency", 5.0),
            get_param("depth", 0.5),
            sample_rate,
        ))),
        "Bandpass Filter" => Ok(Box::new(BandPass::new(
            get_param("low", 500.0),
            get_param("high", 1500.0),
            sample_rate,
        ))),
        "Clipper" => Ok(Box::new(Clipper::new(get_param("max_ampl", 1.0)))),
        "Moving Average" => Ok(Box::new(MovingAverage::new(
            get_param("size", 10.0) as usize
        ))),
        "Delay" => Ok(Box::new(DelayFilter::new(
            sample_rate,
            get_param("delay_for", 0.5),
        ))),
        "Compressor" => Ok(Box::new(Compressor::default())),
        other => Err(format!("Unknown filter type: {}", other)),
    }
}

/// Handle all graph commands.
///
/// Structural variants (AddNode, RemoveNode, Connect, Disconnect) mutate
/// `GraphData` only. Playback variants (Play, Pause, Stop, SetParameter)
/// also send `AudioMessage`s to the render thread.
fn handle_graph_command(
    cmd: GraphCommand,
    graph_system: &mut GraphData,
    shared_state: &Arc<SharedAudioState>,
    message_tx: &crossbeam::channel::Sender<AudioMessage>,
    event_tx: &Sender<BackendEvent>,
) {
    match cmd {
        // -- Structural --
        GraphCommand::AddNode {
            id,
            node_type,
            kind,
            ..
        } => {
            match kind {
                NodeKind::Generator => {
                    log::info!("Adding generator ({node_type}) to graph system");
                    let source = create_source(&node_type, &[]).unwrap();
                    let idx = graph_system.system.add_source(source);
                    graph_system.source_map.insert(id, idx);
                }
                NodeKind::Filter => {
                    log::info!("Adding filter ({node_type}) to graph system");
                    let sample_rate = shared_state.sample_rate.load(Ordering::Relaxed) as f32;
                    let filter = create_filter(&node_type, &[], sample_rate).unwrap();
                    let idx = graph_system.system.add_filter(filter);
                    graph_system.filter_map.insert(id, idx);
                }
                NodeKind::Sink => {
                    log::info!("Adding sink ({node_type}) to graph system");
                    let sink = Box::new(AudioOutputSink::new());
                    let idx = graph_system.system.add_sink(sink);
                    graph_system.sink_map.insert(id, idx);
                }
            }
            rebuild_and_swap(graph_system, message_tx, event_tx);
        }
        GraphCommand::RemoveNode { id } => {
            if let Some(idx) = graph_system.filter_map.remove(&id) {
                graph_system.system.remove_filter(idx);
            } else if let Some(idx) = graph_system.source_map.remove(&id) {
                graph_system.system.remove_source(idx);
            } else if let Some(idx) = graph_system.sink_map.remove(&id) {
                graph_system.system.remove_sink(idx);
            }
            rebuild_and_swap(graph_system, message_tx, event_tx);
        }
        GraphCommand::Connect {
            from,
            from_port,
            to,
            to_port,
        } => {
            let from_is_source = graph_system.source_map.contains_key(&from);
            let to_is_sink = graph_system.sink_map.contains_key(&to);

            if from_is_source && !to_is_sink {
                log::info!("Connecting source element {from} to graph element {to}");
                let src_idx = graph_system.source_map[&from];
                let filter_idx = graph_system.filter_map[&to];
                graph_system
                    .system
                    .connect_source(src_idx, filter_idx, to_port);
            } else if !from_is_source && to_is_sink {
                log::info!("Connecting graph element {from} to sink {to}");
                let filter_idx = graph_system.filter_map[&from];
                let sink_idx = graph_system.sink_map[&to];
                graph_system
                    .system
                    .connect_sink(filter_idx, sink_idx, from_port);
            } else if !from_is_source && !to_is_sink {
                log::info!("Connecting graph element {from} to graph element {to}");
                let from_idx = graph_system.filter_map[&from];
                let to_idx = graph_system.filter_map[&to];
                graph_system
                    .system
                    .connect(from_idx, to_idx, from_port, to_port);
            } else {
                log::warn!("Unhandled connection source?: {from_is_source} to sink? {to_is_sink}");
            }
            rebuild_and_swap(graph_system, message_tx, event_tx);
        }
        GraphCommand::Disconnect { from, to } => {
            if let (Some(&from_idx), Some(&to_idx)) = (
                graph_system.filter_map.get(&from),
                graph_system.filter_map.get(&to),
            ) {
                let _ = graph_system.system.disconnect(from_idx, to_idx);
            }
            rebuild_and_swap(graph_system, message_tx, event_tx);
        }

        // -- Playback control --
        GraphCommand::SetParameter {
            node_id,
            param_name,
            value,
        } => {
            if let Some(&idx) = graph_system.filter_map.get(&node_id) {
                let _ = message_tx.send(AudioMessage::Graph(GraphAudioMessage::SetParameter {
                    node_index: idx.index(),
                    param_name,
                    value,
                }));
            }
        }
        GraphCommand::StartNode { id } => {
            if let Some(&idx) = graph_system.source_map.get(&id) {
                let _ = message_tx.send(AudioMessage::Graph(GraphAudioMessage::StartSource {
                    source_index: idx,
                }));
            }
        }
        GraphCommand::StopNode { id } => {
            if let Some(&idx) = graph_system.source_map.get(&id) {
                let _ = message_tx.send(AudioMessage::Graph(GraphAudioMessage::StopSource {
                    source_index: idx,
                }));
            }
        }
    }
}

/// Rebuild the system after structural changes: compute topology, swap to render thread, set graph mode.
fn rebuild_and_swap(
    graph_system: &mut GraphData,
    message_tx: &crossbeam::channel::Sender<AudioMessage>,
    event_tx: &Sender<BackendEvent>,
) {
    match graph_system.system.compute() {
        Ok(()) => {
            let cloned = graph_system.system.clone();
            let _ = message_tx.send(AudioMessage::Graph(GraphAudioMessage::Swap(cloned)));
            let _ = message_tx.send(AudioMessage::SetRenderMode(RenderMode::Graph));
        }
        Err(e) => {
            let _ = event_tx.send(BackendEvent::GraphError {
                description: format!("{:?}", e),
            });
        }
    }
}

/// Spawns the command processing thread
///
/// This thread:
/// - Receives commands from the frontend
/// - Validates commands
/// - Updates app state
/// - Translates commands to audio messages
/// - Sends audio messages to the render thread
/// - Reports errors and events back to the frontend
pub fn spawn_command_thread(
    mut app: App,
    shared_state: Arc<SharedAudioState>,
    command_rx: Receiver<Command>,
    event_tx: Sender<BackendEvent>,
    message_tx: crossbeam::channel::Sender<AudioMessage>,
) -> JoinHandle<()> {
    thread::Builder::new()
        .name("command-processor".to_string())
        .spawn(move || {
            log::info!("Command thread started");

            let mut graph_system = GraphData::default();

            loop {
                match command_rx.recv() {
                    // Shutdown has highest priority — exit the loop immediately
                    Ok(Command::Audio(AudioCommand::Shutdown)) => {
                        log::info!("Shutdown command received");
                        shared_state.shutdown.store(true, Ordering::Release);
                        let _ = message_tx.send(AudioMessage::Shutdown);
                        let _ = event_tx.send(BackendEvent::AudioStopped);
                        break;
                    }

                    // Audio commands: validate + translate in one step
                    Ok(Command::Audio(cmd)) => match cmd.into_audio_message(&app) {
                        Ok(msg) => {
                            if message_tx.send(msg).is_err() {
                                log::warn!("Audio message channel closed");
                            }
                        }
                        Err(e) => {
                            let _ = event_tx.send(BackendEvent::CommandError {
                                command: "AudioCommand".to_string(),
                                error: e.to_string(),
                            });
                            log::warn!("Audio command failed: {}", e);
                        }
                    },

                    // Graph commands: unified handler for structural + playback
                    Ok(Command::Graph(cmd)) => {
                        handle_graph_command(
                            cmd,
                            &mut graph_system,
                            &shared_state,
                            &message_tx,
                            &event_tx,
                        );
                    }

                    // App commands: validate then apply to app state
                    Ok(Command::App(cmd)) => {
                        if let Err(e) = cmd.validate(&app) {
                            let _ = event_tx.send(BackendEvent::CommandError {
                                command: format!("{:?}", cmd),
                                error: e.to_string(),
                            });
                            log::warn!("App command validation failed: {:?} - {}", cmd, e);
                            continue;
                        }
                        app.on_event(cmd);
                    }

                    Err(_) => {
                        log::info!("Command channel closed");
                        break;
                    }
                }
            }

            log::info!("Command thread shutting down");
        })
        .expect("Failed to spawn command thread")
}
