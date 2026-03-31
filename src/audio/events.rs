//! Backend event types, category filtering, and the filtered event sender.

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::Sender;

use serde::{Deserialize, Serialize};

// Categories

/// Broad category of a [`BackendEvent`]. Used to opt in or out of event streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventCategory {
    /// Lifecycle events: audio started/stopped, output device changes.
    Status,
    /// High-frequency audio data chunks, intended for waveform display or recording.
    /// Disable this category in production to avoid flooding the event channel.
    Audio,
    /// Performance counters: CPU usage, latency, buffer underruns.
    Diagnostics,
    /// Errors, panics, and graph compilation failures.
    Error,
}

impl EventCategory {
    fn bit(self) -> u8 {
        match self {
            Self::Status => 1 << 0,
            Self::Audio => 1 << 1,
            Self::Diagnostics => 1 << 2,
            Self::Error => 1 << 3,
        }
    }
}

// Filter

/// Controls which [`EventCategory`]s are forwarded to the caller.
///
/// The default enables [`Status`](EventCategory::Status) and
/// [`Error`](EventCategory::Error) only, skipping the high-frequency
/// [`Audio`](EventCategory::Audio) and [`Diagnostics`](EventCategory::Diagnostics)
/// streams.
///
/// # Example
/// ```
/// use rustic::audio::{EventFilter, EventCategory};
///
/// let filter = EventFilter::default()
///     .with(EventCategory::Audio)         // enable waveform chunks
///     .with(EventCategory::Diagnostics);  // enable metrics
/// ```
#[derive(Debug, Clone)]
pub struct EventFilter {
    pub(crate) enabled: u8,
}

impl EventFilter {
    /// Enable all event categories.
    pub fn all() -> Self {
        Self { enabled: 0xFF }
    }

    /// Disable all event categories (silence everything).
    pub fn none() -> Self {
        Self { enabled: 0 }
    }

    /// Enable an additional category.
    pub fn with(mut self, category: EventCategory) -> Self {
        self.enabled |= category.bit();
        self
    }

    /// Disable a category.
    pub fn without(mut self, category: EventCategory) -> Self {
        self.enabled &= !category.bit();
        self
    }

    /// Returns `true` if the given category is currently enabled.
    pub fn allows(&self, category: EventCategory) -> bool {
        self.enabled & category.bit() != 0
    }
}

impl Default for EventFilter {
    fn default() -> Self {
        Self::none()
            .with(EventCategory::Status)
            .with(EventCategory::Error)
    }
}

// Event leaf types

/// Lifecycle events emitted by the engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StatusEvent {
    /// Audio engine started successfully.
    AudioStarted { sample_rate: u32 },
    /// Audio engine stopped (clean shutdown).
    AudioStopped,
    /// List of available output devices.
    OutputDeviceList { devices: Vec<String> },
    /// Active output device changed.
    OutputDeviceChanged { device: String },
}

/// High-frequency audio data, emitted after each rendered block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioEvent {
    /// Stereo-interleaved samples `[L, R, L, R, …]` from the last render block.
    /// Use `.step_by(2)` to extract L or R channel.
    Chunk(Vec<f32>),
}

/// Performance and diagnostic counters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiagnosticsEvent {
    /// Periodic CPU and latency snapshot.
    Metrics { cpu_usage: f32, latency_ms: f32 },
    /// The CPAL callback found the ring buffer empty; filled with silence.
    BufferUnderrun { count: u64 },
}

/// Error and failure events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorEvent {
    /// A command could not be processed (validation or routing failure).
    CommandFailed { command: String, message: String },
    /// Audio graph topology is invalid or failed to compile.
    GraphError { description: String },
    /// An audio thread panicked; the thread name and panic message are included.
    ThreadPanic { thread: String, message: String },
}

// Top-level envelope

/// Top-level backend event, categorised for filtering.
///
/// Wrap your event channel receiver with a match on the outer variant to
/// process only the categories you care about.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackendEvent {
    Status(StatusEvent),
    Audio(AudioEvent),
    Diagnostics(DiagnosticsEvent),
    Error(ErrorEvent),
}

impl BackendEvent {
    /// Returns the [`EventCategory`] of this event.
    pub fn category(&self) -> EventCategory {
        match self {
            Self::Status(_) => EventCategory::Status,
            Self::Audio(_) => EventCategory::Audio,
            Self::Diagnostics(_) => EventCategory::Diagnostics,
            Self::Error(_) => EventCategory::Error,
        }
    }
}

// Filtered sender

/// Wraps `mpsc::Sender<BackendEvent>` with a live-updateable category filter.
///
/// Cheap to clone — all clones share the same atomic filter state.
#[derive(Clone)]
pub(crate) struct EventSender {
    tx: Sender<BackendEvent>,
    filter: Arc<AtomicU8>,
}

impl EventSender {
    pub fn new(tx: Sender<BackendEvent>, filter: EventFilter) -> Self {
        Self {
            tx,
            filter: Arc::new(AtomicU8::new(filter.enabled)),
        }
    }

    /// Send an event; silently drops it if its category is not enabled.
    pub fn send(&self, event: BackendEvent) {
        if self.filter.load(Ordering::Relaxed) & event.category().bit() != 0 {
            let _ = self.tx.send(event);
        }
    }

    /// Update the enabled categories at runtime.
    #[allow(dead_code)]
    pub fn set_filter(&self, filter: EventFilter) {
        self.filter.store(filter.enabled, Ordering::Relaxed);
    }
}
