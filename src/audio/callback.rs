//! cpal audio callback implementation

use crossbeam::queue::ArrayQueue;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::shared_state::SharedAudioState;
use crate::core::audio::CHANNELS;

const UNDERRUN_FADE_FRAMES: usize = 64;

fn fill_output(audio_queue: &ArrayQueue<f32>, data: &mut [f32]) -> bool {
    let available = audio_queue.len().min(data.len());
    if available >= data.len() {
        for sample in data {
            *sample = audio_queue.pop().unwrap_or(0.0);
        }
        return false;
    }

    data.fill(0.0);
    let available_frames = available / CHANNELS;
    let fade_frames = available_frames.min(UNDERRUN_FADE_FRAMES);
    let fade_start = available_frames.saturating_sub(fade_frames);
    for (index, sample) in data.iter_mut().take(available).enumerate() {
        let frame = index / CHANNELS;
        let gain = if fade_frames > 0 && frame >= fade_start {
            (available_frames.saturating_sub(frame + 1)) as f32 / fade_frames as f32
        } else {
            1.0
        };
        *sample = audio_queue.pop().unwrap_or(0.0) * gain;
    }
    true
}

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
        if fill_output(&audio_queue, data) {
            shared_state
                .buffer_underruns
                .fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_buffer_is_copied_without_a_fade() {
        let queue = ArrayQueue::new(8);
        for sample in [0.5, -0.5, 0.25, -0.25] {
            queue.push(sample).unwrap();
        }
        let mut output = [0.0; 4];
        assert!(!fill_output(&queue, &mut output));
        assert_eq!(output, [0.5, -0.5, 0.25, -0.25]);
    }

    #[test]
    fn partial_stereo_buffer_fades_to_silence() {
        let queue = ArrayQueue::new(16);
        for _ in 0..4 {
            queue.push(1.0).unwrap();
            queue.push(-1.0).unwrap();
        }
        let mut output = [9.0; 12];
        assert!(fill_output(&queue, &mut output));
        assert!(output[0].abs() > output[4].abs());
        assert_eq!(&output[6..], &[0.0; 6]);
        assert!(queue.is_empty());
    }
}
