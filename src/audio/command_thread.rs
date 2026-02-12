//! Command processing thread implementation

use crate::app::commands::SystemCommand;
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
    next_graph_id: u64,
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

    let mut generator: MultiToneGenerator = ToneGeneratorBuilder::new()
        .waveform(waveform)
        .frequency(freq)
        .build()
        .into();
    generator.start();

    Ok(simple_source(generator))
}

fn create_filter(node_type: &str, params: &[(String, f32)]) -> Result<Box<dyn Filter>, String> {
    let get_param = |name: &str, default: f32| -> f32 {
        params
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| *v)
            .unwrap_or(default)
    };

    match node_type {
        "Low Pass Filter" => Ok(Box::new(LowPassFilter::new(get_param(
            "cutoff_frequency",
            1000.0,
        )))),
        "High Pass Filter" => Ok(Box::new(HighPassFilter::new(get_param(
            "cutoff_frequency",
            1000.0,
        )))),
        "Gain" => Ok(Box::new(GainFilter::new(get_param("factor", 1.0)))),
        // TODO: all other filter types
        other => Err(format!("Unknown filter type: {}", other)),
    }
}

fn handle_graph_command(
    graph_cmd: GraphCommand,
    graph_system: &mut GraphData,
    message_tx: &crossbeam::channel::Sender<AudioMessage>,
    event_tx: &Sender<BackendEvent>,
) {
    match graph_cmd {
        GraphCommand::AddNode {
            node_type, kind, ..
        } => {
            match kind {
                NodeKind::Generator => {
                    let source = create_source(&node_type, &[]).unwrap();
                    let idx = graph_system.system.add_source(source);
                    graph_system
                        .source_map
                        .insert(graph_system.next_graph_id, idx);
                }
                NodeKind::Filter => {
                    let filter = create_filter(&node_type, &[]).unwrap();
                    let idx = graph_system.system.add_filter(filter);
                    graph_system
                        .filter_map
                        .insert(graph_system.next_graph_id, idx);
                }
                NodeKind::Sink => {
                    let sink = Box::new(AudioOutputSink::new());
                    let idx = graph_system.system.add_sink(sink);
                    graph_system
                        .sink_map
                        .insert(graph_system.next_graph_id, idx);
                }
            }
            graph_system.next_graph_id += 1;
        }
        GraphCommand::RemoveNode { id } => {
            if let Some(idx) = graph_system.filter_map.remove(&id) {
                graph_system.system.remove_filter(idx);
            } else if let Some(idx) = graph_system.source_map.remove(&id) {
                graph_system.system.remove_source(idx);
            } else if let Some(idx) = graph_system.sink_map.remove(&id) {
                graph_system.system.remove_sink(idx);
            }
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
                let src_idx = graph_system.source_map[&from];
                let filter_idx = graph_system.filter_map[&to];
                graph_system
                    .system
                    .connect_source(src_idx, filter_idx, to_port);
            } else if !from_is_source && to_is_sink {
                let filter_idx = graph_system.filter_map[&from];
                let sink_idx = graph_system.sink_map[&to];
                graph_system
                    .system
                    .connect_sink(filter_idx, sink_idx, from_port);
            } else if !from_is_source && !to_is_sink {
                let from_idx = graph_system.filter_map[&from];
                let to_idx = graph_system.filter_map[&to];
                graph_system
                    .system
                    .connect(from_idx, to_idx, from_port, to_port);
            }
        }
        GraphCommand::Disconnect { from, to } => {
            if let (Some(&from_idx), Some(&to_idx)) = (
                graph_system.filter_map.get(&from),
                graph_system.filter_map.get(&to),
            ) {
                let _ = graph_system.system.disconnect(from_idx, to_idx);
            }
        }
        GraphCommand::SetParameter {
            node_id,
            param_name,
            value,
        } => {
            // If currently playing, send real-time update to render thread
            if let Some(&idx) = graph_system.filter_map.get(&node_id) {
                let _ = message_tx.send(AudioMessage::Graph(GraphAudioMessage::SetParameter {
                    node_index: idx.index(),
                    param_name,
                    value,
                }));
            }
        }
        GraphCommand::SetNodePosition { .. } => {
            // Position is frontend-only state, nothing to do here
        }
        GraphCommand::Play => {
            // Compute topology and send to render thread
            match graph_system.system.compute() {
                Ok(()) => {
                    // TODO: clone/rebuild System for the render thread
                    // let _ = message_tx.send(AudioMessage::SwapGraph(Box::new(system_clone)));
                    let _ = message_tx.send(AudioMessage::SetRenderMode(RenderMode::Graph));
                }
                Err(e) => {
                    let _ = event_tx.send(BackendEvent::GraphError {
                        description: format!("{:?}", e),
                    });
                }
            }
        }
        GraphCommand::Pause | GraphCommand::Stop => {
            let _ = message_tx.send(AudioMessage::SetRenderMode(RenderMode::Instruments));
            let _ = message_tx.send(AudioMessage::Graph(GraphAudioMessage::Clear));
        }
        GraphCommand::SaveGraph(_) | GraphCommand::LoadGraph(_) => {
            // Save/load is handled by the frontend graph library serializing its own state.
            // The command thread rebuilds the System from the sequence of AddNode/Connect
            // commands that the frontend replays on load.
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
                    Ok(Command::System(SystemCommand::Quit)) => {
                        log::info!("Quit command received");
                        shared_state.shutdown.store(true, Ordering::Release);
                        let _ = message_tx.send(AudioMessage::Shutdown);
                        let _ = event_tx.send(BackendEvent::AudioStopped);
                        break;
                    }
                    Ok(Command::Graph(cmd)) => {
                        // Manage graph commands
                        handle_graph_command(cmd, &mut graph_system, &message_tx, &event_tx);
                    }
                    Ok(cmd) => {
                        // Validate command
                        if let Err(e) = cmd.validate(&app) {
                            let _ = event_tx.send(BackendEvent::CommandError {
                                command: format!("{:?}", cmd),
                                error: e.to_string(),
                            });
                            log::warn!("Command validation failed: {:?} - {}", cmd, e);
                            continue;
                        }

                        // Update app state
                        app.on_event(cmd.clone());

                        // Translate to audio message
                        if let Some(msg) = cmd.translate_to_audio_message(&mut app)
                            && message_tx.send(msg.clone()).is_err()
                        {
                            // Channel closed - audio thread has shut down
                            log::warn!("Audio message channel closed, dropping command: {:?}", cmd);
                        }
                    }
                    Err(_) => {
                        log::info!("Command channel closed");
                        break;
                    } // Channel closed
                }
            }

            log::info!("Command thread shutting down");
        })
        .expect("Failed to spawn command thread")
}
