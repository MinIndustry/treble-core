//! Input handling module for hardware devices
//!
//! This module provides abstractions for handling input from various hardware
//! devices like keyboards, MIDI controllers, etc.

#[cfg(feature = "input")]
pub mod keyboard;

#[cfg(not(feature = "input"))]
pub mod keyboard {
    //! Stub keyboard module for when input features are disabled

    /// Stub function that returns None when input features are disabled
    pub fn find_keyboard() -> Option<()> {
        None
    }
}
