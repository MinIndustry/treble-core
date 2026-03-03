//! Audio render thread implementation

use super::config::AudioConfig;
use super::messages::{AudioMessage, InstrumentAudioMessage};
use super::shared_state::SharedAudioState;
use crate::audio::BackendEvent;
use crate::core::graph::System;
use crate::instruments::Instrument;
use crossbeam::queue::ArrayQueue;
use petgraph::graph::NodeIndex;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use super::{GraphAudioMessage, RenderMode};

/// Spawns the audio render thread that generates audio samples
///
/// This thread:
/// - Processes control messages from the command thread
/// - Generates audio by calling instrument methods
/// - Writes audio to the ring buffer for the cpal callback to consume
///
/// # Examples
///
/// ```no_run
/// use rustic::audio::render_thread::spawn_audio_render_thread;
/// ```
pub fn spawn_audio_render_thread(
    shared_state: Arc<SharedAudioState>,
    instruments: Vec<Box<dyn Instrument + Send + Sync>>,
    message_rx: crossbeam::channel::Receiver<AudioMessage>,
    audio_queue: Arc<ArrayQueue<f32>>,
    config: AudioConfig,
    event_tx: Sender<BackendEvent>,
) -> JoinHandle<()> {
    thread::Builder::new()
        .name("audio-render".to_string())
        .spawn(move || {
            let mut instruments = instruments;
            let mut system: Option<System> = None;
            // Buffer holds interleaved stereo samples (L, R, L, R, …) for both render modes
            let mut chunk_buffer =
                Vec::with_capacity(config.render_chunk_size * crate::core::audio::CHANNELS);
            let mut render_mode = RenderMode::Instruments; // Default to instrument render

            let sample_rate = shared_state.sample_rate.load(Ordering::Relaxed);
            // target_samples accounts for stereo interleaving (2 samples per frame)
            let target_samples =
                config.calculate_ring_buffer_size(sample_rate) * crate::core::audio::CHANNELS;

            while !shared_state.shutdown.load(Ordering::Relaxed) {
                // Process all pending control messages
                while let Ok(msg) = message_rx.try_recv() {
                    process_audio_message(&mut instruments, &mut system, &mut render_mode, msg);
                }

                // Throttle render thread at target latency worth of samples
                if audio_queue.len() >= target_samples {
                    thread::sleep(Duration::from_micros(100));
                    continue;
                }

                // Generate audio chunk (always stereo-interleaved: L, R, L, R, …)
                match render_mode {
                    RenderMode::Instruments => {
                        chunk_buffer.clear();
                        let n = instruments.len().max(1) as f32;
                        for _ in 0..config.render_chunk_size {
                            instruments.iter_mut().for_each(|inst| inst.tick());
                            let mono = instruments
                                .iter_mut()
                                .map(|inst| inst.get_output())
                                .sum::<f32>()
                                / n;
                            chunk_buffer.push(mono); // L
                            chunk_buffer.push(mono); // R
                        }
                    }
                    RenderMode::Graph => {
                        if let Some(ref mut system) = system {
                            system.run();
                            if let Ok(sink) = system.get_sink(0) {
                                let frames = sink.consume();
                                chunk_buffer.clear();
                                for frame in &frames {
                                    chunk_buffer.push(frame[0]); // L
                                    chunk_buffer.push(frame[1]); // R
                                }
                            } else {
                                chunk_buffer.clear();
                            }
                        } else {
                            chunk_buffer.clear();
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

                // Broadcast chunk for recording / analysis (best-effort, ignore send errors)
                let _ = event_tx.send(BackendEvent::AudioChunk(chunk_buffer.clone()));
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
        GraphAudioMessage::StartSource { source_index } => {
            if let Some(system) = system {
                system.start_source(source_index);
            }
        }
        GraphAudioMessage::StopSource { source_index } => {
            if let Some(system) = system {
                system.stop_source(source_index);
            }
        }
        GraphAudioMessage::SetParameter {
            node_index,
            param_name,
            value,
        } => {
            if let Some(system) = system
                && let Some(f) = system.get_filter_mut(NodeIndex::new(node_index))
            {
                f.set_parameter(param_name.as_str(), value);
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
            log::info!("Render mode changed to: {}", mode);
            *render_mode = mode;
        }
        AudioMessage::Shutdown => {
            // Will be handled by the shutdown flag
        }
    }
}
