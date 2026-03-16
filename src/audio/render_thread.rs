//! Audio render thread implementation

use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crossbeam::queue::ArrayQueue;
use petgraph::graph::NodeIndex;

use super::config::AudioConfig;
use super::events::{AudioEvent, BackendEvent, ErrorEvent, EventSender};
use super::messages::{AudioMessage, GraphAudioMessage, InstrumentAudioMessage};
use super::shared_state::SharedAudioState;
use crate::core::graph::System;

/// Spawns the audio render thread.
///
/// The thread owns a single [`System`] graph (always valid — never `Option`).
/// It runs the system block-by-block, processing control messages between blocks.
///
/// If the render loop panics (e.g. a DSP node hits an unrecoverable state), the
/// panic is caught, an [`ErrorEvent::ThreadPanic`] event is emitted, and the
/// thread exits cleanly rather than taking down the whole process.
pub(crate) fn spawn_audio_render_thread(
    shared_state: Arc<SharedAudioState>,
    mut system: System,
    message_rx: crossbeam::channel::Receiver<AudioMessage>,
    audio_queue: Arc<ArrayQueue<f32>>,
    config: AudioConfig,
    event_tx: EventSender,
) -> JoinHandle<()> {
    thread::Builder::new()
        .name("audio-render".to_string())
        .spawn(move || {
            let result = panic::catch_unwind(AssertUnwindSafe(|| {
                render_loop(
                    &shared_state,
                    &mut system,
                    &message_rx,
                    &audio_queue,
                    &config,
                    &event_tx,
                );
            }));

            if let Err(payload) = result {
                let message = payload
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_else(|| "unknown panic".to_string());

                log::error!("audio-render thread panicked: {message}");
                event_tx.send(BackendEvent::Error(ErrorEvent::ThreadPanic {
                    thread: "audio-render".to_string(),
                    message,
                }));
            }

            log::info!("Audio render thread shut down");
        })
        .expect("Failed to spawn audio render thread")
}

fn render_loop(
    shared_state: &Arc<SharedAudioState>,
    system: &mut System,
    message_rx: &crossbeam::channel::Receiver<AudioMessage>,
    audio_queue: &Arc<ArrayQueue<f32>>,
    config: &AudioConfig,
    event_tx: &EventSender,
) {
    let mut chunk_buffer =
        Vec::with_capacity(config.render_chunk_size * crate::core::audio::CHANNELS);

    let sample_rate = shared_state.sample_rate.load(Ordering::Relaxed);
    let target_samples =
        config.calculate_ring_buffer_size(sample_rate) * crate::core::audio::CHANNELS;

    let mut block_count: u64 = 0;

    while !shared_state.shutdown.load(Ordering::Relaxed) {
        // Process all pending control messages
        while let Ok(msg) = message_rx.try_recv() {
            process_audio_message(system, msg, event_tx);
        }

        // Throttle to target latency
        if audio_queue.len() >= target_samples {
            thread::sleep(Duration::from_micros(100));
            continue;
        }

        // Run the graph for one block
        system.run();
        chunk_buffer.clear();
        match system.get_sink(0) {
            Ok(sink) => {
                let frames = sink.consume();
                log::trace!("[render] consumed {} frames from sink", frames.len());
                for frame in &frames {
                    chunk_buffer.push(frame[0]); // L
                    chunk_buffer.push(frame[1]); // R
                }
            }
            Err(e) => {
                log::warn!("[render] get_sink(0) failed: {e}");
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

        block_count += 1;
        // Every ~1 second (86 blocks @ 512 frames / 44100 Hz), log a status line
        if block_count.is_multiple_of(86) {
            let max_sample = chunk_buffer.iter().cloned().fold(0.0_f32, f32::max);
            let active_sources = (0..system.sources_len())
                .filter(|&i| system.is_source_active(i))
                .count();
            log::info!(
                "[render] block={block_count} queue={}/{} chunk={} samples max={:.4} active_sources={active_sources}/{}",
                audio_queue.len(),
                target_samples,
                chunk_buffer.len(),
                max_sample,
                system.sources_len()
            );
        }

        // Broadcast chunk for recording / analysis
        event_tx.send(BackendEvent::Audio(AudioEvent::Chunk(chunk_buffer.clone())));
    }
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

fn process_graph_message(system: &mut System, cmd: GraphAudioMessage, event_tx: &EventSender) {
    match cmd {
        GraphAudioMessage::Swap(new_system) => {
            *system = new_system;
        }
        GraphAudioMessage::Clear => {
            *system = System::silent();
        }
        GraphAudioMessage::StartSource { source_index } => {
            log::info!(
                "[render] StartSource source_index={source_index} (system has {} sources)",
                system.sources_len()
            );
            system.start_source(source_index);
            log::info!(
                "[render] source {source_index} is_active={}",
                system.is_source_active(source_index)
            );
        }
        GraphAudioMessage::StopSource { source_index } => {
            log::info!("[render] StopSource source_index={source_index}");
            system.stop_source(source_index);
        }
        GraphAudioMessage::KillSource { source_index } => {
            log::info!("[render] KillSource source_index={source_index}");
            system.kill_source(source_index);
        }
        GraphAudioMessage::SetParameter {
            node_index,
            param_name,
            value,
        } => {
            if param_name == "mix_mode" {
                let mode = rustic_meta::MixMode::from_ordinal(value as usize);
                system.set_mix_mode(NodeIndex::new(node_index), mode);
            } else if let Some(f) = system.get_filter_mut(NodeIndex::new(node_index)) {
                f.set_parameter(param_name.as_str(), value);
            }
        }
        GraphAudioMessage::SetSourceParameter {
            source_index,
            param_name,
            value,
        } => {
            system.set_source_parameter(source_index, param_name.as_str(), value);
        }
        GraphAudioMessage::AddModulation {
            from_source,
            target,
            param_name,
        } => {
            system.add_mod_wire(from_source, target, param_name);
        }
        GraphAudioMessage::RemoveModulation {
            from_source,
            target,
            param_name,
        } => {
            system.remove_mod_wire(from_source, &target, &param_name);
        }
    }
    // Suppress unused parameter warning when no variant uses event_tx yet
    let _ = event_tx;
}

fn process_audio_message(system: &mut System, msg: AudioMessage, event_tx: &EventSender) {
    match msg {
        AudioMessage::Instrument(cmd) => process_instrument_message(system, cmd),
        AudioMessage::Graph(cmd) => process_graph_message(system, cmd, event_tx),
        AudioMessage::Shutdown => {
            // Handled via shutdown flag in shared_state
        }
    }
}
