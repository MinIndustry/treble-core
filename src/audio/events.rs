//! Backend events sent from audio system to frontend

/// Events sent from backend to frontend for status updates and error reporting
#[derive(Debug, Clone)]
pub enum BackendEvent {
    // Status updates
    AudioStarted { sample_rate: u32 },
    AudioStopped,

    // Error reporting
    CommandError { command: String, error: String },
    BufferUnderrun { count: u64 },

    // Diagnostics
    Metrics { cpu_usage: f32, latency_ms: f32 },
}
