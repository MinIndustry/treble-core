//! Audio render thread implementation

use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crossbeam::queue::ArrayQueue;
use petgraph::graph::NodeIndex;

use super::config::AudioConfig;
use super::events::{AudioEvent, BackendEvent, ErrorEvent, EventCategory, EventSender};
use super::messages::{AudioMessage, GraphAudioMessage};
use super::resampler::OutputResampler;
use super::scheduler::{EventScheduler, apply_instrument_message, render_block};
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
) -> std::io::Result<JoinHandle<()>> {
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
    let engine_sample_rate = shared_state.engine_sample_rate.load(Ordering::Relaxed);
    let output_sample_rate = shared_state.sample_rate.load(Ordering::Relaxed);
    let mut resampler = OutputResampler::new(engine_sample_rate, output_sample_rate);
    let mut output_buffer = Vec::with_capacity(
        ((config.render_chunk_size as f64 * output_sample_rate as f64
            / engine_sample_rate.max(1) as f64)
            .ceil() as usize
            + 2)
            * crate::core::audio::CHANNELS,
    );

    let target_samples =
        config.calculate_ring_buffer_size(output_sample_rate) * crate::core::audio::CHANNELS;

    let mut block_count: u64 = 0;
    let mut scheduler = EventScheduler::new();
    let mut current_frame: u64 = 0;
    // Edge-triggered so TRBC-RT-001 logs once per failure, not per block.
    let mut volume_sync_failed = false;

    while !shared_state.shutdown.load(Ordering::Relaxed) {
        // Process all pending control messages
        while let Ok(msg) = message_rx.try_recv() {
            process_audio_message(system, &mut scheduler, msg, event_tx);
        }

        // Throttle to target latency. Sleep for most of the time the device
        // callback will take to drain back to the watermark, instead of
        // spin-polling at 100us (~10,000 wakeups/s of idle CPU — BN-006).
        // The clamp keeps control-message latency bounded: 8ms is well under
        // the ring's ~93ms capacity, and note events are frame-scheduled ahead
        // of time so they do not depend on this loop's reaction speed.
        let queued = audio_queue.len();
        if queued >= target_samples {
            let surplus_frames = (queued - target_samples) / crate::core::audio::CHANNELS;
            let drain_micros = surplus_frames as u64 * 1_000_000 / output_sample_rate.max(1) as u64;
            thread::sleep(Duration::from_micros(drain_micros.clamp(1_000, 8_000)));
            continue;
        }

        // Sync master volume to the sink each block (cheap atomic read).
        // The sink applies it before limiting, so the limiter always acts as
        // a hard ceiling regardless of the volume setting.
        //
        // The set must happen outside any debug_assert — a side effect inside
        // one vanishes from release builds, taking the volume control with
        // it. A silent system has no sink yet and nothing to apply the volume
        // to (the next compiled graph picks it up), but a *present* sink 0
        // refusing the parameter is a wiring bug: log it once (this runs per
        // block) and trip debug builds.
        if system.sinks_len() > 0
            && system
                .set_sink_parameter(
                    0,
                    "master_volume",
                    shared_state.master_volume.load(Ordering::Relaxed),
                )
                .is_err()
        {
            if !volume_sync_failed {
                volume_sync_failed = true;
                log::error!(
                    "TRBC-RT-001: sink 0 refused master_volume — the master level control \
                     is not reaching the audio output"
                );
            }
            debug_assert!(false, "TRBC-RT-001: sink 0 refused master_volume");
        } else {
            volume_sync_failed = false;
        }

        // Run the graph for one block, splitting at scheduled note events so
        // each event lands on exactly its frame. Master volume + limiting are
        // applied inside the sink; render_block consumes it per segment.
        chunk_buffer.clear();
        current_frame = render_block(system, &mut scheduler, current_frame, &mut chunk_buffer);
        shared_state
            .current_frame
            .store(current_frame, Ordering::Relaxed);

        // A sweep is applied every block, so the System reports a filter that
        // refuses the parameter once per graph instead of once per block.
        for message in system.take_automation_warnings() {
            event_tx.send(BackendEvent::Error(ErrorEvent::CommandFailed {
                command: "Automation".into(),
                message,
            }));
        }

        // Convert stable engine-rate audio to the active output-device rate.
        resampler.process(&chunk_buffer, &mut output_buffer);

        // Write device-rate audio to the ring buffer.
        let mut written = 0;
        for &sample in &output_buffer {
            if audio_queue.push(sample).is_ok() {
                written += 1;
            } else {
                break;
            }
        }
        if written != output_buffer.len() {
            log::warn!(
                "Failed to write full chunk: {} / {}",
                written,
                output_buffer.len()
            );
        }

        block_count += 1;
        // Every ~1 second (86 blocks @ 512 frames / 44100 Hz), log a status line
        if block_count.is_multiple_of(86) {
            let max_sample = chunk_buffer.iter().cloned().fold(0.0_f32, f32::max);
            let active_sources = (0..system.sources_len())
                .filter(|&i| system.is_source_active(i))
                .count();
            log::trace!(
                "[render] block={block_count} queue={}/{} chunk={} samples max={:.4} active_sources={active_sources}/{}",
                audio_queue.len(),
                target_samples,
                chunk_buffer.len(),
                max_sample,
                system.sources_len()
            );
        }

        if event_tx.allows(EventCategory::Audio) {
            event_tx.send(BackendEvent::Audio(AudioEvent::Chunk(chunk_buffer.clone())));
            let stems = (1..system.sinks_len())
                .map(|index| {
                    system
                        .get_sink(index)
                        .map(|sink| {
                            sink.consume()
                                .into_iter()
                                .flat_map(|frame| frame.into_iter())
                                .collect()
                        })
                        .unwrap_or_default()
                })
                .collect();
            event_tx.send(BackendEvent::Audio(AudioEvent::StemChunk(stems)));
        } else {
            for index in 1..system.sinks_len() {
                if let Ok(sink) = system.get_sink(index) {
                    sink.discard();
                }
            }
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
                let mode = treble_meta::MixMode::from_ordinal(value as usize);
                system.set_mix_mode(NodeIndex::new(node_index), mode);
            } else if let Some(f) = system.get_filter_mut(NodeIndex::new(node_index)) {
                if !f.set_parameter(param_name.as_str(), value) {
                    event_tx.send(BackendEvent::Error(ErrorEvent::CommandFailed {
                        command: "SetParameter".into(),
                        message: format!(
                            "unknown parameter '{param_name}' for filter node {node_index}"
                        ),
                    }));
                }
            } else {
                event_tx.send(BackendEvent::Error(ErrorEvent::CommandFailed {
                    command: "SetParameter".into(),
                    message: format!("filter node {node_index} does not exist"),
                }));
            }
        }
        GraphAudioMessage::SetSourceParameter {
            source_index,
            param_name,
            value,
        } => {
            if let Err(error) =
                system.set_source_parameter(source_index, param_name.as_str(), value)
            {
                event_tx.send(BackendEvent::Error(ErrorEvent::CommandFailed {
                    command: "SetSourceParameter".into(),
                    message: error.to_string(),
                }));
            }
        }
        GraphAudioMessage::AddModulation {
            from_source,
            target,
            param_name,
        } => {
            if let Err(error) = system.add_mod_wire(from_source, target, param_name) {
                event_tx.send(BackendEvent::Error(ErrorEvent::CommandFailed {
                    command: "AddModulation".into(),
                    message: error.to_string(),
                }));
            }
        }
        GraphAudioMessage::RemoveModulation {
            from_source,
            target,
            param_name,
        } => {
            system.remove_mod_wire(from_source, &target, &param_name);
        }
    }
}

fn process_audio_message(
    system: &mut System,
    scheduler: &mut EventScheduler,
    msg: AudioMessage,
    event_tx: &EventSender,
) {
    match msg {
        // Untimestamped events keep the old behavior: applied at block start.
        AudioMessage::Instrument(cmd) => apply_instrument_message(system, cmd),
        AudioMessage::ScheduledInstrument { at_frame, command } => {
            scheduler.schedule(at_frame, command);
        }
        AudioMessage::ScheduledGraphSwap {
            at_frame,
            system,
            fade_in_frames,
            tail_frames,
        } => {
            scheduler.schedule_graph_swap(at_frame, system, fade_in_frames, tail_frames);
        }
        AudioMessage::Graph(cmd) => {
            // Immediate graph replacement starts a new graph generation.
            if matches!(cmd, GraphAudioMessage::Clear | GraphAudioMessage::Swap(_)) {
                scheduler.clear();
            }
            process_graph_message(system, cmd, event_tx);
        }
        AudioMessage::Shutdown => {
            // Handled via shutdown flag in shared_state
        }
    }
}
