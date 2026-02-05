//! Rustic — Core synthesis library
//!
//! # Overview
//! Rustic is the workspace's core crate providing the DSP building blocks used by
//! frontends and examples. It exposes composable primitives for sound generation
//! (oscillators, noise), envelopes (ADSR, segments), filters (LP/HP, band-pass,
//! tremolo, delays), and signal graph utilities. The crate is intentionally
//! frontend-agnostic so it can be used by GUI frontends, headless tools, and
//! testing utilities alike.
//!
//! # Purpose
//! - Provide clear, composable primitives for building instruments and audio
//!   processing pipelines.
//! - Offer a canonical API for frontends (Tauri GUI, CLI) to build on.
//! - Contain reusable utilities (notes, tone tables, plotting helpers) for both
//!   musical and DSP workflows.
//!
//! # Quick example
//! ```rust
//! use rustic::prelude::*;
//! use rustic::core::generator::prelude::Waveform;
//! // Build a simple sine tone generator (see `core::generator` docs for details)
//! let mut gen = core::generator::prelude::builder::ToneGeneratorBuilder::new().build();
//! gen.start();
//! let sample = gen.tick(1.0 / 44100.0);
//! ```
//!
//! # Features
//! - `plotting`: utilities to render and inspect signal plots
//! - `meta`: filter/instrument metadata used by GUI frontends
//!
//! # Audio I/O
//! See `start_app` for an example that uses `cpal` for audio output and a simple
//! audio callback loop used in the examples folder.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use log::error;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::JoinHandle;

pub mod app;

/// Audio subsystem with three-thread architecture for real-time safe audio
pub mod audio;

/// The core module of rustic. Contains the envelopes, filters, generators and the
/// graph building utilities.
pub mod core;

/// Input handling module for hardware devices
pub mod inputs;

/// Instruments are structures that implement the `Instrument` trait.
pub mod instruments;

#[cfg(feature = "meta")]
/// This module defines the metadata structures for the application.
/// It allows to store and retreive metadata about filters
pub mod meta;

/// The mod score contains all the building block for creating music
/// Sheets contain instruments layed out on a staff, divided into measures
/// Notes in the measures are structures that implement the `MeasureNote` trait.
/// This allows to build complex notes, chords, ...
pub mod score;

const APP_ID: (&str, &str, &str) = ("rustic", "minigrim0", "xyz");

/// Main prelude module that exports the most commonly used types from the crate
pub mod prelude {
    // App exports
    pub use super::app::{
        self,
        prelude::{App, Commands},
    };

    // Core exports - only expose the module, details accessed through it
    pub use super::core;

    // Score exports (legacy) - export individual legacy score types into prelude so tests and callers keep working
    pub use super::score::prelude::{
        Chord, ChordModifier, DurationModifier, Measure, Note, NoteDuration, NoteModifier,
        NoteName, Score, Staff, StaffInstance, TimeSignature,
    };

    // Instruments exports
    pub use super::instruments::Instrument;
}

#[cfg(feature = "plotting")]
pub mod plotting;

#[cfg(feature = "testing")]
pub mod testing;

// Re-export Note from core utils
pub use core::{Note, NOTES};

/// Initialize logging based on configuration
pub fn init_logging(
    config: &audio::LogConfig,
    config_dir: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use simplelog::*;

    let log_level = match config.level.as_str() {
        "trace" => LevelFilter::Trace,
        "debug" => LevelFilter::Debug,
        "info" => LevelFilter::Info,
        "warn" => LevelFilter::Warn,
        "error" => LevelFilter::Error,
        _ => LevelFilter::Info,
    };

    let log_config = ConfigBuilder::new().set_time_format_rfc3339().build();

    let mut loggers: Vec<Box<dyn SharedLogger>> = vec![];

    if config.log_to_stdout {
        loggers.push(TermLogger::new(
            log_level,
            log_config.clone(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ));
    }

    if config.log_to_file {
        let log_path = config_dir.join(&config.log_file);
        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;
        loggers.push(WriteLogger::new(log_level, log_config, log_file));
    }

    CombinedLogger::init(loggers)?;
    Ok(())
}

/// Handle to the audio system threads
pub struct AudioHandle {
    command_thread: JoinHandle<()>,
    render_thread: JoinHandle<()>,
    stream: cpal::Stream,
    shared_state: std::sync::Arc<audio::SharedAudioState>,
}

impl AudioHandle {
    /// Gracefully shutdown the audio system
    pub fn shutdown(self) -> Result<(), audio::AudioError> {
        use std::sync::atomic::Ordering;

        // Signal shutdown
        self.shared_state.shutdown.store(true, Ordering::Release);

        // Wait for threads to finish
        self.command_thread
            .join()
            .map_err(|_| audio::AudioError::ThreadPanic)?;
        self.render_thread
            .join()
            .map_err(|_| audio::AudioError::ThreadPanic)?;

        // Drop stream (stops playback)
        drop(self.stream);

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

pub fn start_app(
    event_tx: Sender<audio::BackendEvent>,
    command_rx: Receiver<app::prelude::Commands>,
) -> Result<AudioHandle, audio::AudioError> {
    use std::sync::atomic::Ordering;
    use std::sync::Arc;

    // Initialize app (loads config from file)
    let mut app = app::prelude::App::new();

    // Initialize logging from config
    if let Ok(config_dir) = app::prelude::FSConfig::app_root_dir() {
        let _ = init_logging(&app.config.logging, &config_dir);
    }

    log::info!("Starting Rustic audio system");

    // Validate audio config
    app.config
        .audio
        .validate()
        .map_err(audio::AudioError::ConfigError)?;

    let config = app.config.audio.clone();
    let shared_state = Arc::new(audio::SharedAudioState::new());

    // Create lock-free queues using crossbeam
    use crossbeam::queue::ArrayQueue;
    let audio_queue = Arc::new(ArrayQueue::<f32>::new(config.audio_ring_buffer_size));
    let audio_queue_producer = audio_queue.clone();
    let audio_queue_consumer = audio_queue;

    let (message_tx, message_rx) = crossbeam::channel::bounded(config.message_ring_buffer_size);

    // Move instruments to audio thread
    let instruments = std::mem::take(&mut app.instruments);

    // Setup cpal
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or(audio::AudioError::NoDevice)?;

    let mut supported_configs_range = device
        .supported_output_configs()
        .map_err(|e| audio::AudioError::StreamError(e.to_string()))?;

    let supported_config = supported_configs_range
        .next()
        .ok_or(audio::AudioError::StreamError(
            "No supported config".to_string(),
        ))?
        .with_max_sample_rate();

    let mut cpal_config = supported_config.config();
    cpal_config.buffer_size = cpal::BufferSize::Fixed(config.cpal_buffer_size as u32);

    let sample_rate = cpal_config.sample_rate.0;
    app.config.system.sample_rate = sample_rate;
    shared_state
        .sample_rate
        .store(sample_rate, Ordering::Relaxed);

    log::info!(
        "Audio configuration: sample_rate={}, buffer_size={}, ring_buffer={}",
        sample_rate,
        config.cpal_buffer_size,
        config.audio_ring_buffer_size
    );

    // Spawn threads
    let render_thread = audio::spawn_audio_render_thread(
        shared_state.clone(),
        instruments,
        message_rx,
        audio_queue_producer,
        config.clone(),
    );

    let command_thread = audio::spawn_command_thread(
        app,
        shared_state.clone(),
        command_rx,
        event_tx.clone(),
        message_tx,
    );

    // Build cpal stream
    let callback = audio::create_cpal_callback(audio_queue_consumer, shared_state.clone());

    let stream = device
        .build_output_stream(
            &cpal_config,
            callback,
            move |err| {
                error!("Audio stream error: {}", err);
            },
            None,
        )
        .map_err(|e| audio::AudioError::StreamError(e.to_string()))?;

    stream
        .play()
        .map_err(|e| audio::AudioError::StreamError(e.to_string()))?;

    // Notify frontend
    let _ = event_tx.send(audio::BackendEvent::AudioStarted { sample_rate });

    log::info!("Audio system started successfully");

    Ok(AudioHandle {
        command_thread,
        render_thread,
        stream,
        shared_state,
    })
}
