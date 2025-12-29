//! Core utilities module
//!
//! ## Overview
//! This module contains shared types and helpers used throughout the core DSP
//! system including musical note types, tone frequency tables, and keyboard
//! mapping utilities.
//!
//! ## Notes & Tones
//! - `Note`: musical note representation (name, octave, modifiers)
//! - `NOTES` / `TONES_FREQ`: precomputed frequency tables used for note-to-frequency
//!   lookups. Frequencies are provided per semitone and octave to simplify
//!   instrument construction.
//!
//! ## Input helpers
//! Keyboard-related types and key mappings are provided to support the
//! `inputs` module and example frontends.

/// Musical tone definitions and frequency mappings
pub mod tones;

/// Keyboard input mapping and key definitions
pub mod keys;

/// Utility macros for audio processing
pub mod macros;

/// Musical note representation
pub mod note;

// Re-export commonly used types
pub use note::Note;
pub use tones::{NOTES, TONES_FREQ};

// Re-export key types for input handling
pub use keys::{EventType, Key, KeyCode, KeyType};
