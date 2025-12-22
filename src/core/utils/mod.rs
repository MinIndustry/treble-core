//! Core utilities module
//!
//! This module contains shared utility functions and types used throughout
//! the core audio processing system.

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
