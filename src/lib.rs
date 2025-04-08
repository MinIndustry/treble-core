/// The `app` module contains the main application data structures and functions.
/// It provides CLI utilities for managing the application as well as filesystem
/// utilities for managing files and directories.
mod app;
pub mod core;
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
mod score;

const APP_ID: (&str, &str, &str) = ("rustic", "minigrim0", "xyz");

pub mod prelude {
    pub use super::app::prelude::App;
    pub use super::core;
    pub use super::score::*;
}

use crate::core::generator::{Bendable, ToneGenerator, VariableFrequency};
use core::tones::NOTES;

#[cfg(feature = "plotting")]
pub mod plotting;

#[cfg(test)]
pub mod tests;

pub trait KeyboardGenerator: ToneGenerator + VariableFrequency + Bendable + Send + Sync {}

/// A note with its octave
#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone)]
pub struct Note(pub NOTES, pub u8);
