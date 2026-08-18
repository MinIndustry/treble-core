//! Backend event types, category filtering, and the filtered event sender.

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::sync::mpsc::{RecvError, RecvTimeoutError, TryRecvError};
use std::time::{Duration, Instant};

use crossbeam::channel::{Receiver, Sender, TrySendError, bounded, select_biased, unbounded};

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
/// use treble::audio::{EventFilter, EventCategory};
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
    /// One stereo-interleaved block per instrument slot, before master limiting.
    /// Vector indices match the application audio-graph slot indices.
    StemChunk(Vec<Vec<f32>>),
}

/// Performance and diagnostic counters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiagnosticsEvent {
    /// Periodic CPU and latency snapshot.
    Metrics { cpu_usage: f32, latency_ms: f32 },
    /// The CPAL callback found the ring buffer empty; filled with silence.
    BufferUnderrun { count: u64 },
    /// High-frequency audio events discarded because their bounded queue was full.
    AudioEventsDropped { count: u64 },
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

/// Receiver that keeps reliable events separate from bounded, lossy audio payloads.
pub struct EventReceiver {
    reliable_rx: Receiver<BackendEvent>,
    audio_rx: Receiver<BackendEvent>,
    dropped_audio_events: Arc<AtomicU64>,
    reported_audio_events: AtomicU64,
}

impl EventReceiver {
    fn dropped_event(&self) -> Option<BackendEvent> {
        let count = self.dropped_audio_events.load(Ordering::Relaxed);
        let reported = self.reported_audio_events.swap(count, Ordering::Relaxed);
        (count > reported).then_some(BackendEvent::Diagnostics(
            DiagnosticsEvent::AudioEventsDropped {
                count: count - reported,
            },
        ))
    }

    pub fn try_recv(&self) -> Result<BackendEvent, TryRecvError> {
        let reliable_disconnected = match self.reliable_rx.try_recv() {
            Ok(event) => return Ok(event),
            Err(crossbeam::channel::TryRecvError::Empty) => false,
            Err(crossbeam::channel::TryRecvError::Disconnected) => true,
        };
        if let Some(event) = self.dropped_event() {
            return Ok(event);
        }
        match self.audio_rx.try_recv() {
            Ok(event) => Ok(event),
            Err(crossbeam::channel::TryRecvError::Empty) => {
                if reliable_disconnected {
                    Err(TryRecvError::Disconnected)
                } else {
                    Err(TryRecvError::Empty)
                }
            }
            Err(crossbeam::channel::TryRecvError::Disconnected) => {
                if reliable_disconnected {
                    Err(TryRecvError::Disconnected)
                } else {
                    Err(TryRecvError::Empty)
                }
            }
        }
    }

    pub fn recv(&self) -> Result<BackendEvent, RecvError> {
        if let Some(event) = self.dropped_event() {
            return Ok(event);
        }
        select_biased! {
            recv(self.reliable_rx) -> event => match event {
                Ok(event) => Ok(event),
                Err(_) => self.audio_rx.recv().map_err(|_| RecvError),
            },
            recv(self.audio_rx) -> event => match event {
                Ok(event) => Ok(event),
                Err(_) => self.reliable_rx.recv().map_err(|_| RecvError),
            },
        }
    }

    pub fn recv_timeout(&self, timeout: Duration) -> Result<BackendEvent, RecvTimeoutError> {
        if let Some(event) = self.dropped_event() {
            return Ok(event);
        }
        let deadline = Instant::now() + timeout;
        select_biased! {
            recv(self.reliable_rx) -> event => match event {
                Ok(event) => Ok(event),
                Err(_) => self.audio_rx.recv_timeout(deadline.saturating_duration_since(Instant::now())).map_err(map_timeout_error),
            },
            recv(self.audio_rx) -> event => match event {
                Ok(event) => Ok(event),
                Err(_) => self.reliable_rx.recv_timeout(deadline.saturating_duration_since(Instant::now())).map_err(map_timeout_error),
            },
            default(timeout) => Err(RecvTimeoutError::Timeout),
        }
    }

    pub fn dropped_audio_events(&self) -> u64 {
        self.dropped_audio_events.load(Ordering::Relaxed)
    }
}

fn map_timeout_error(error: crossbeam::channel::RecvTimeoutError) -> RecvTimeoutError {
    match error {
        crossbeam::channel::RecvTimeoutError::Timeout => RecvTimeoutError::Timeout,
        crossbeam::channel::RecvTimeoutError::Disconnected => RecvTimeoutError::Disconnected,
    }
}

/// Routes reliable events through an unbounded control channel and audio payloads
/// through a bounded drop-oldest channel.
///
/// Cheap to clone — all clones share the same atomic filter state.
#[derive(Clone)]
pub(crate) struct EventSender {
    reliable_tx: Sender<BackendEvent>,
    audio_tx: Sender<BackendEvent>,
    audio_rx: Receiver<BackendEvent>,
    filter: Arc<AtomicU8>,
    dropped_audio_events: Arc<AtomicU64>,
}

impl EventSender {
    pub fn new(filter: EventFilter, audio_capacity: usize) -> (Self, EventReceiver) {
        let (reliable_tx, reliable_rx) = unbounded();
        let (audio_tx, audio_rx) = bounded(audio_capacity.max(1));
        let dropped_audio_events = Arc::new(AtomicU64::new(0));
        let sender = Self {
            reliable_tx,
            audio_tx,
            audio_rx: audio_rx.clone(),
            filter: Arc::new(AtomicU8::new(filter.enabled)),
            dropped_audio_events: Arc::clone(&dropped_audio_events),
        };
        let receiver = EventReceiver {
            reliable_rx,
            audio_rx,
            dropped_audio_events,
            reported_audio_events: AtomicU64::new(0),
        };
        (sender, receiver)
    }

    /// Send an event; silently drops it if its category is not enabled.
    pub fn send(&self, event: BackendEvent) {
        if !self.allows(event.category()) {
            return;
        }
        if matches!(event, BackendEvent::Audio(_)) {
            match self.audio_tx.try_send(event) {
                Ok(()) => {}
                Err(TrySendError::Full(event)) => {
                    let _ = self.audio_rx.try_recv();
                    if self.audio_tx.try_send(event).is_err() {
                        log::trace!("audio event queue remained full; dropped newest payload");
                    }
                    self.dropped_audio_events.fetch_add(1, Ordering::Relaxed);
                }
                Err(TrySendError::Disconnected(_)) => {}
            }
        } else {
            let _ = self.reliable_tx.send(event);
        }
    }

    pub fn allows(&self, category: EventCategory) -> bool {
        self.filter.load(Ordering::Relaxed) & category.bit() != 0
    }

    /// Update the enabled categories at runtime.
    pub fn set_filter(&self, filter: EventFilter) {
        self.filter.store(filter.enabled, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_queue_drops_oldest_without_losing_reliable_events() {
        let (sender, receiver) = EventSender::new(EventFilter::all(), 2);
        sender.send(BackendEvent::Audio(AudioEvent::Chunk(vec![1.0])));
        sender.send(BackendEvent::Audio(AudioEvent::Chunk(vec![2.0])));
        sender.send(BackendEvent::Audio(AudioEvent::Chunk(vec![3.0])));
        sender.send(BackendEvent::Status(StatusEvent::AudioStopped));

        assert!(matches!(receiver.try_recv(), Ok(BackendEvent::Status(_))));
        assert!(matches!(
            receiver.try_recv(),
            Ok(BackendEvent::Diagnostics(
                DiagnosticsEvent::AudioEventsDropped { count: 1 }
            ))
        ));
        assert!(matches!(
            receiver.try_recv(),
            Ok(BackendEvent::Audio(AudioEvent::Chunk(samples))) if samples == vec![2.0]
        ));
        assert_eq!(receiver.dropped_audio_events(), 1);
    }

    #[test]
    fn disabled_audio_is_never_queued() {
        let (sender, receiver) = EventSender::new(EventFilter::default(), 1);
        sender.send(BackendEvent::Audio(AudioEvent::Chunk(vec![1.0])));
        assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
        assert_eq!(receiver.dropped_audio_events(), 0);
    }
}
