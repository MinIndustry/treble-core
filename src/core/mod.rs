//! Core audio processing module
//!
//! This module contains the fundamental audio processing components:
//! generators, envelopes, filters, and signal graph processing.

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
pub use utils::{Note, NOTES};
