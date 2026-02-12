//! Audio render thread implementation

use super::config::AudioConfig;
use super::messages::{AudioMessage, InstrumentAudioMessage};
use super::shared_state::SharedAudioState;
use crate::core::graph::System;
use crate::instruments::Instrument;
use crossbeam::queue::ArrayQueue;
use petgraph::graph::NodeIndex;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use super::{GraphAudioMessage, RenderMode};

/// Spawns the audio render thread that generates audio samples
///
/// This thread:
/// - Processes control messages from the command thread
/// - Generates audio by calling instrument methods
/// - Writes audio to the ring buffer for the cpal callback to consume
pub fn spawn_audio_render_thread(
    shared_state: Arc<SharedAudioState>,
    instruments: Vec<Box<dyn Instrument + Send + Sync>>,
    message_rx: crossbeam::channel::Receiver<AudioMessage>,
    audio_queue: Arc<ArrayQueue<f32>>,
    config: AudioConfig,
) -> JoinHandle<()> {
    thread::Builder::new()
        .name("audio-render".to_string())
        .spawn(move || {
            let mut instruments = instruments;
            let mut system: Option<System> = None;
            let mut chunk_buffer = vec![0.0f32; config.render_chunk_size];
            let mut render_mode = RenderMode::Instruments; // Default to instrument render

            while !shared_state.shutdown.load(Ordering::Relaxed) {
                // Process all pending control messages
                while let Ok(msg) = message_rx.try_recv() {
                    process_audio_message(&mut instruments, &mut system, &mut render_mode, msg);
                }

                // Check if ring buffer has space
                if audio_queue.len() + config.render_chunk_size > audio_queue.capacity() {
                    thread::sleep(Duration::from_micros(100));
                    continue;
                }

                // Generate audio chunk
                match render_mode {
                    RenderMode::Instruments => {
                        for sample in chunk_buffer.iter_mut() {
                            instruments.iter_mut().for_each(|inst| inst.tick());
                            *sample = instruments
                                .iter_mut()
                                .map(|inst| inst.get_output())
                                .sum::<f32>();
                        }
                    }
                    RenderMode::Graph => {
                        if let Some(ref mut system) = system {
                            for sample in chunk_buffer.iter_mut() {
                                system.run();
                                if let Ok(sink) = system.get_sink(0) {
                                    let values = sink.consume(1);
                                    *sample = values.first().copied().unwrap_or(0.0);
                                } else {
                                    *sample = 0.0;
                                }
                            }
                        } else {
                            chunk_buffer.fill(0.0);
                        }
                    }
                }

                // Write to ring buffer (lock-free)
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
            }

            log::info!("Audio render thread shutting down");
        })
        .expect("Failed to spawn audio render thread")
}

fn process_instrument_message(
    command: InstrumentAudioMessage,
    instruments: &mut [Box<dyn Instrument + Send + Sync>],
) {
    match command {
        InstrumentAudioMessage::NoteStart {
            instrument_idx,
            note,
            velocity,
        } => {
            if let Some(inst) = instruments.get_mut(instrument_idx) {
                inst.start_note(note, velocity);
            } else {
                log::warn!("Invalid instrument index: {}", instrument_idx);
            }
        }
        InstrumentAudioMessage::NoteStop {
            instrument_idx,
            note,
        } => {
            if let Some(inst) = instruments.get_mut(instrument_idx) {
                inst.stop_note(note);
            } else {
                log::warn!("Invalid instrument index: {}", instrument_idx);
            }
        }
        InstrumentAudioMessage::SetOctave { .. } => {
            // Octave is managed by the command thread, not needed here
        }
    }
}

fn process_graph_message(command: GraphAudioMessage, system: &mut Option<System>) {
    match command {
        GraphAudioMessage::Swap(new_graph) => {
            *system = Some(new_graph);
        }
        GraphAudioMessage::Clear => {
            *system = None;
        }
        GraphAudioMessage::SetParameter {
            node_index,
            param_name,
            value,
        } => {
            if let Some(system) = system {
                if let Some(f) = system.get_filter_mut(NodeIndex::new(node_index)) {
                    #[cfg(feature = "meta")]
                    f.set_parameter(param_name.as_str(), value);
                    #[cfg(not(feature = "meta"))]
                    log::warn!(
                        "SetParameter requires the 'meta' feature (node={}, param={})",
                        node_index,
                        param_name
                    );
                }
            }
        }
    }
}

/// Process a single audio control message
fn process_audio_message(
    instruments: &mut [Box<dyn Instrument + Send + Sync>],
    system: &mut Option<System>,
    render_mode: &mut RenderMode,
    msg: AudioMessage,
) {
    match msg {
        AudioMessage::Instrument(cmd) => process_instrument_message(cmd, instruments),
        AudioMessage::Graph(cmd) => process_graph_message(cmd, system),
        AudioMessage::SetRenderMode(mode) => {
            *render_mode = mode;
        }
        AudioMessage::SetMasterVolume { .. } => {
            // Volume is applied in the command thread
        }
        AudioMessage::SetSampleRate { .. } => {
            // Sample rate changes require restarting the audio system
        }
        AudioMessage::Shutdown => {
            // Will be handled by the shutdown flag
        }
        _ => {
            // TODO
        }
    }
}
