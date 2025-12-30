//! Error types for audio system

use thiserror::Error;

/// Errors that can occur in the audio system
#[derive(Debug, Error)]
pub enum AudioError {
    #[error("No audio device available")]
    NoDevice,

    #[error("Failed to build audio stream: {0}")]
    StreamError(String),

    #[error("Thread panicked")]
    ThreadPanic,

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Errors that can occur when validating commands
#[derive(Debug, Error)]
pub enum CommandError {
    #[error("Row index out of bounds: {0}")]
    RowOutOfBounds(u8),

    #[error("Invalid octave: {0} (must be 0-8)")]
    InvalidOctave(u8),

    #[error("Invalid volume: {0} (must be 0.0-1.0)")]
    InvalidVolume(f32),
}
