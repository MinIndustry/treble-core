//! Core audio processing module
//!
//! ## Overview
//! The `core` module provides the foundational DSP abstractions and primitives
//! used across the workspace: `Generator`s (oscillators and tone combiners),
//! `Envelope`s (ADSR and segment-based curves), `Filter`s (frequency and time
//! domain processors), and graph utilities for routing signals between nodes.
//!
//! ## Purpose
//! This module exists to expose composable, well-tested building blocks so
//! frontends and higher-level APIs (in `instruments` and `score`) can assemble
//! musical behaviours without re-implementing low-level DSP logic.
//!
//! ## Key components
//! - `generator`: Oscillators, noise sources and multi-tone combiners.
//! - `envelope`: Time-based modulation (ADSR, segments) that shapes amplitude or
//!   other parameters.
//! - `filters`: Audio processors (amplifiers, tremolo, delays, resonant filters).
//! - `graph`: Utilities to wire processors together and build signal graphs.
//! - `utils`: Shared types, e.g. `Note` and tone frequency tables.
//!
//! ## Usage
//! Prefer using `core::prelude` for common types. See module docs for examples
//! and mathematical foundations (e.g., sine wave generation and ADSR equations).

/// Defines the different envelope shapes & types
/// Envelopes implement the `Envelope` trait and can
/// be of 3 types; linear, bezier, adsr
pub mod envelope;

/// Filters are structures that implement the `Filter` trait. They
/// operate on audio signals to modify their frequency response.
/// examples include low-pass, high-pass, tremolo, ...
pub mod filters;

/// Generators are structures that implement the `Generator` trait.
/// They generate audio signals of different types such as sine, square, sawtooth, etc.
pub mod generator;

/// Audio signal graph processing and routing
pub mod graph;

/// Core utilities including note types, tones, and helper functions
pub mod utils;

// Re-export commonly used types from utils
pub use utils::{NOTES, Note};
