//! Audio subsystem
//!
//! Three-component architecture:
//! - `App` (main thread): compiles the `AudioGraph`, sends `AudioMessage`s directly
//! - Render thread: owns a `System`, runs it block-by-block
//! - cpal callback: copies pre-rendered audio from the ring buffer to hardware

pub mod callback;
pub mod config;
pub mod error;
pub mod events;
mod handle;
pub mod messages;
pub(crate) mod render_thread;
pub mod shared_state;

// Re-export commonly used types
pub use callback::create_cpal_callback;
pub use config::{AudioConfig, LogConfig};
pub use error::{AudioError, CommandError};
pub use events::{
    AudioEvent, BackendEvent, DiagnosticsEvent, ErrorEvent, EventCategory, EventFilter, StatusEvent,
};
pub(crate) use events::EventSender;
pub use handle::{AudioHandle, AudioMetrics};
pub use messages::{AudioMessage, GraphAudioMessage, InstrumentAudioMessage};
pub use shared_state::SharedAudioState;
