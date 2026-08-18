use super::{AudioError, SharedAudioState};
use std::sync::Arc;
use std::thread::JoinHandle;

use super::stream::OutputStreamRuntime;

/// Handle to the audio render thread.
///
/// The platform stream lives on a dedicated thread because some backends do
/// not allow it to be moved between threads. Shutdown joins both owners.
pub struct AudioHandle {
    render_thread: JoinHandle<()>,
    output_stream: OutputStreamRuntime,
    shared_state: Arc<SharedAudioState>,
}

impl AudioHandle {
    pub(crate) fn new(
        render_thread: JoinHandle<()>,
        output_stream: OutputStreamRuntime,
        shared_state: Arc<SharedAudioState>,
    ) -> Self {
        Self {
            render_thread,
            output_stream,
            shared_state,
        }
    }

    /// Gracefully shut down the audio system.
    pub fn shutdown(self) -> Result<(), AudioError> {
        use std::sync::atomic::Ordering;

        self.shared_state.shutdown.store(true, Ordering::Release);

        self.render_thread
            .join()
            .map_err(|_| AudioError::ThreadPanic)?;
        self.output_stream.shutdown();

        Ok(())
    }

    /// Access the shared audio state (e.g. to update master_volume).
    pub fn shared_state(&self) -> &Arc<SharedAudioState> {
        &self.shared_state
    }

    /// Get audio metrics.
    pub fn get_metrics(&self) -> AudioMetrics {
        use std::sync::atomic::Ordering;

        AudioMetrics {
            buffer_underruns: self.shared_state.buffer_underruns.load(Ordering::Relaxed),
            sample_rate: self.shared_state.sample_rate.load(Ordering::Relaxed),
            engine_sample_rate: self.shared_state.engine_sample_rate.load(Ordering::Relaxed),
        }
    }
}

/// Audio system metrics.
pub struct AudioMetrics {
    pub buffer_underruns: u64,
    pub sample_rate: u32,
    pub engine_sample_rate: u32,
}
