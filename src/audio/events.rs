//! Backend events sent from audio system to frontend

use serde::{Deserialize, Serialize};

/// Events sent from backend to frontend for status updates and error reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackendEvent {
    // Status updates
    AudioStarted { sample_rate: u32 },
    AudioStopped,

    // Error reporting
    CommandError { command: String, error: String },
    BufferUnderrun { count: u64 },

    // Diagnostics
    Metrics { cpu_usage: f32, latency_ms: f32 },
    OutputDeviceList { devices: Vec<String> },
    OutputDeviceChanged { device: String },
    GraphError { description: String },
}
