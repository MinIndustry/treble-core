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
//! use rustic::core::generator::prelude::*;
//! // Build a simple sine tone generator (see `core::generator` docs for details)
//! let mut generator = builder::ToneGeneratorBuilder::new().build();
//! generator.start();
//! let _sample = generator.tick(1.0 / 44100.0);
//! ```
//! # Working with sound
//!
//! To work with sound it is recommended to create an instance of the rustic app:
//!
//! ```rust
//! use rustic::prelude::*;
//! use rustic::audio::EventFilter;
//!
//! let mut app = App::new();
//!
//! // Then start the app
//! let event_rx = match app.start(EventFilter::default()) {
//!     Ok(rx) => rx,
//!     Err(e) => {
//!         panic!("Unable to start the rustic engine: {e}");
//!     }
//! };
//! ```
//!
//! This event_rx is a std::sync::mspc::Receiver channel sending `BackendEvent`
//! Events sent through this channel are filtered with the EventFilter given as parameter
//! in the start function
//!
//! Adding instruments at runtime requires a call to [`recompile()`](crate::prelude::App::recompile)
//!
//! # Features
//! - `plotting`: utilities to render and inspect signal plots
//! - `ts`: enables transpilation of the metadata structures for generators and filters to
//! TypeScript

pub mod app;

/// Audio subsystem with three-thread architecture for real-time safe audio
pub mod audio;

/// The core module of rustic. Contains the envelopes, filters, generators and the
/// graph building utilities.
pub mod core;

/// Instruments are structures that implement the `Instrument` trait.
pub mod instruments;

/// This module defines the metadata structures for the application.
/// It allows to store and retrieve metadata about filters
pub mod meta;

/// The mod score contains all the building block for creating music
/// Sheets contain instruments laid out on a staff, divided into measures
/// Notes in the measures are structures that implement the `MeasureNote` trait.
/// This allows to build complex notes, chords, ...
pub mod score;

/// Main prelude module that exports the most commonly used types from the crate
pub mod prelude {
    // App exports
    pub use super::app::{
        self,
        prelude::{App, AppCommand, AudioCommand, Command},
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

pub const APP_ID: (&str, &str, &str) = ("rustic", "minigrim0", "xyz");

// Re-export Note from core utils
pub use core::{NOTES, Note};

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

    let mut loggers: Vec<Box<dyn SharedLogger>> = vec![];

    if config.log_to_stdout {
        let term_config = ConfigBuilder::new()
            .set_time_format_rfc3339()
            .set_target_level(LevelFilter::Info)
            .set_location_level(LevelFilter::Debug)
            .build();

        loggers.push(TermLogger::new(
            log_level,
            term_config,
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ));
    }

    if config.log_to_file {
        let file_config = ConfigBuilder::new()
            .set_time_format_rfc3339()
            .set_target_level(LevelFilter::Trace)
            .set_location_level(LevelFilter::Trace)
            .build();

        let log_path = config_dir.join(&config.log_file);
        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;
        loggers.push(WriteLogger::new(log_level, file_config, log_file));
    }

    CombinedLogger::init(loggers)?;
    Ok(())
}
