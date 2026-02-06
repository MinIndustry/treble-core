//! cpal audio callback implementation

use crossbeam::queue::ArrayQueue;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::shared_state::SharedAudioState;

/// Creates a cpal audio callback that reads from the ring buffer
///
/// This callback:
/// - Runs in the real-time audio thread (highest priority)
/// - Only reads from the ring buffer and copies to output
/// - NO allocations, NO locks, NO complex logic
/// - Falls back to silence on buffer underrun
pub fn create_cpal_callback(
    audio_queue: Arc<ArrayQueue<f32>>,
    shared_state: Arc<SharedAudioState>,
) -> impl FnMut(&mut [f32], &cpal::OutputCallbackInfo) + Send + 'static {
    move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
        let available = audio_queue.len();

        if available >= data.len() {
            // Happy path: read pre-rendered audio
            for sample in data.iter_mut() {
                *sample = audio_queue.pop().unwrap_or(0.0);
            }
        } else {
            // Buffer underrun: fill with silence
            data.fill(0.0);
            shared_state
                .buffer_underruns
                .fetch_add(1, Ordering::Relaxed);

            // Also try to read whatever is available to keep buffer clean
            for sample in data.iter_mut().take(available) {
                *sample = audio_queue.pop().unwrap_or(0.0);
            }
        }
    }
}
