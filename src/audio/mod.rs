//! Audio subsystem with three-thread architecture
//!
//! This module implements a real-time safe audio architecture:
//! - Command thread: validates commands and updates state
//! - Render thread: generates audio samples
//! - cpal callback: copies pre-rendered audio to output
//!
//! The architecture uses lock-free ring buffers for communication
//! and ensures the cpal callback has minimal work to do.

pub mod callback;
pub mod command_thread;
pub mod config;
pub mod error;
pub mod events;
mod handle;
pub mod messages;
pub mod render_thread;
pub mod shared_state;

use serde::{Deserialize, Serialize};

// Re-export commonly used types
pub use callback::create_cpal_callback;
pub use command_thread::spawn_command_thread;
pub use config::{AudioConfig, LogConfig};
pub use error::{AudioError, CommandError};
pub use events::BackendEvent;
pub use handle::{AudioHandle, AudioMetrics};
pub use messages::{AudioMessage, GraphAudioMessage, InstrumentAudioMessage};
pub use render_thread::spawn_audio_render_thread;
pub use shared_state::SharedAudioState;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub enum RenderMode {
    #[default]
    Instruments,
    Graph,
}
