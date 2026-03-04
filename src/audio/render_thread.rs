//! Audio render thread implementation

use super::config::AudioConfig;
use super::messages::{AudioMessage, GraphAudioMessage, InstrumentAudioMessage};
use super::shared_state::SharedAudioState;
use crate::audio::BackendEvent;
use crate::core::graph::System;
use crossbeam::queue::ArrayQueue;
use petgraph::graph::NodeIndex;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Spawns the audio render thread.
///
/// The thread owns a single [`System`] graph (always valid — never `Option`).
/// It runs the system block-by-block, processing control messages between blocks.
/// The initial system is produced by [`crate::app::audio_graph::AudioGraph::compile()`]
/// or [`System::silent()`] if no instruments are loaded yet.
pub fn spawn_audio_render_thread(
    shared_state: Arc<SharedAudioState>,
    mut system: System,
    message_rx: crossbeam::channel::Receiver<AudioMessage>,
    audio_queue: Arc<ArrayQueue<f32>>,
    config: AudioConfig,
    event_tx: Sender<BackendEvent>,
) -> JoinHandle<()> {
    thread::Builder::new()
        .name("audio-render".to_string())
        .spawn(move || {
            let mut chunk_buffer =
                Vec::with_capacity(config.render_chunk_size * crate::core::audio::CHANNELS);

            let sample_rate = shared_state.sample_rate.load(Ordering::Relaxed);
            let target_samples =
                config.calculate_ring_buffer_size(sample_rate) * crate::core::audio::CHANNELS;

            while !shared_state.shutdown.load(Ordering::Relaxed) {
                // Process all pending control messages
                while let Ok(msg) = message_rx.try_recv() {
                    process_audio_message(&mut system, msg);
                }

                // Throttle to target latency
                if audio_queue.len() >= target_samples {
                    thread::sleep(Duration::from_micros(100));
                    continue;
                }

                // Run the graph for one block
                system.run();
                chunk_buffer.clear();
                if let Ok(sink) = system.get_sink(0) {
                    let frames = sink.consume();
                    for frame in &frames {
                        chunk_buffer.push(frame[0]); // L
                        chunk_buffer.push(frame[1]); // R
                    }
                }

                // Write to ring buffer
                let mut written = 0;
                for &sample in chunk_buffer.iter() {
                    if audio_queue.push(sample).is_ok() {
                        written += 1;
                    } else {
                        break;
                    }
                }
                if written != chunk_buffer.len() {
                    log::warn!(
                        "Failed to write full chunk: {} / {}",
                        written,
                        chunk_buffer.len()
                    );
                }

                // Broadcast chunk for recording / analysis
                let _ = event_tx.send(BackendEvent::AudioChunk(chunk_buffer.clone()));
            }

            log::info!("Audio render thread shutting down");
        })
        .expect("Failed to spawn audio render thread")
}

fn process_instrument_message(system: &mut System, cmd: InstrumentAudioMessage) {
    match cmd {
        InstrumentAudioMessage::NoteStart {
            source_index,
            note,
            velocity,
        } => {
            system.start_note(source_index, note, velocity);
        }
        InstrumentAudioMessage::NoteStop { source_index, note } => {
            system.stop_note(source_index, note);
        }
    }
}

fn process_graph_message(system: &mut System, cmd: GraphAudioMessage) {
    match cmd {
        GraphAudioMessage::Swap(new_system) => {
            *system = new_system;
        }
        GraphAudioMessage::Clear => {
            *system = System::silent();
        }
        GraphAudioMessage::StartSource { source_index } => {
            system.start_source(source_index);
        }
        GraphAudioMessage::StopSource { source_index } => {
            system.stop_source(source_index);
        }
        GraphAudioMessage::SetParameter {
            node_index,
            param_name,
            value,
        } => {
            if let Some(f) = system.get_filter_mut(NodeIndex::new(node_index)) {
                f.set_parameter(param_name.as_str(), value);
            }
        }
    }
}

fn process_audio_message(system: &mut System, msg: AudioMessage) {
    match msg {
        AudioMessage::Instrument(cmd) => process_instrument_message(system, cmd),
        AudioMessage::Graph(cmd) => process_graph_message(system, cmd),
        AudioMessage::Shutdown => {
            // Handled via shutdown flag in shared_state
        }
    }
}
