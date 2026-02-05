//! Audio render thread implementation

use crate::instruments::Instrument;
use crossbeam::queue::ArrayQueue;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use super::config::AudioConfig;
use super::messages::AudioMessage;
use super::shared_state::SharedAudioState;

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
            let mut chunk_buffer = vec![0.0f32; config.render_chunk_size];

            while !shared_state.shutdown.load(Ordering::Relaxed) {
                // Process all pending control messages
                while let Ok(msg) = message_rx.try_recv() {
                    process_audio_message(&mut instruments, msg);
                }

                // Check if ring buffer has space
                if audio_queue.len() + config.render_chunk_size > audio_queue.capacity() {
                    thread::sleep(Duration::from_micros(100));
                    continue;
                }

                // Generate audio chunk
                for sample in chunk_buffer.iter_mut() {
                    instruments.iter_mut().for_each(|inst| inst.tick());
                    *sample = instruments
                        .iter_mut()
                        .map(|inst| inst.get_output())
                        .sum::<f32>();
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

/// Process a single audio control message
fn process_audio_message(instruments: &mut [Box<dyn Instrument + Send + Sync>], msg: AudioMessage) {
    match msg {
        AudioMessage::NoteStart {
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
        AudioMessage::NoteStop {
            instrument_idx,
            note,
        } => {
            if let Some(inst) = instruments.get_mut(instrument_idx) {
                inst.stop_note(note);
            } else {
                log::warn!("Invalid instrument index: {}", instrument_idx);
            }
        }
        AudioMessage::SetOctave { .. } => {
            // Octave is managed by the command thread, not needed here
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
    }
}
