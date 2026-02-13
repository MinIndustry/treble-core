use super::{AudioError, SharedAudioState};
use std::sync::Arc;
use std::thread::JoinHandle;

/// Handle to the audio system threads.
///
/// The `cpal::Stream` is intentionally forgotten (leaked) so that this struct
/// is `Send` and can be stored in shared state (e.g. Tauri managed state).
/// The stream continues playing; the OS reclaims it on process exit.
pub struct AudioHandle {
    command_thread: JoinHandle<()>,
    render_thread: JoinHandle<()>,
    shared_state: Arc<SharedAudioState>,
}

impl AudioHandle {
    pub fn new(
        command_thread: JoinHandle<()>,
        render_thread: JoinHandle<()>,
        stream: cpal::Stream,
        shared_state: Arc<SharedAudioState>,
    ) -> Self {
        // Forget the stream so AudioHandle is Send.
        // The stream keeps playing; cleanup happens on process exit.
        std::mem::forget(stream);

        Self {
            command_thread,
            render_thread,
            shared_state,
        }
    }

    /// Gracefully shutdown the audio system
    pub fn shutdown(self) -> Result<(), AudioError> {
        use std::sync::atomic::Ordering;

        // Signal shutdown
        self.shared_state.shutdown.store(true, Ordering::Release);

        // Wait for threads to finish
        self.command_thread
            .join()
            .map_err(|_| AudioError::ThreadPanic)?;
        self.render_thread
            .join()
            .map_err(|_| AudioError::ThreadPanic)?;

        Ok(())
    }

    /// Get audio metrics
    pub fn get_metrics(&self) -> AudioMetrics {
        use std::sync::atomic::Ordering;

        AudioMetrics {
            buffer_underruns: self.shared_state.buffer_underruns.load(Ordering::Relaxed),
            sample_rate: self.shared_state.sample_rate.load(Ordering::Relaxed),
        }
    }
}

/// Audio system metrics
pub struct AudioMetrics {
    pub buffer_underruns: u64,
    pub sample_rate: u32,
}
