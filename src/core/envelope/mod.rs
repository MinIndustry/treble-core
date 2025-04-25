/// The ADSR Envelope module.
mod adsr;

/// The segment envelope module.
/// This module contains the implementation of the segment envelope.
/// The segment is capable of doing either a linear or a bezier interpolation.
mod segment;

/// An envelope that can be used to modulate a signal over time.
/// The base principle is simply to have a function with a varying value over time.
/// This value can then be used to shape either the amplitude, frequency or any other parameter of a sound.
pub trait Envelope: std::fmt::Display + std::fmt::Debug {
    /// Returns the envelope value at the given point in time. The timestamps
    /// is expected to be mapped to the envelope's duration, that is the
    /// minimum value is 0.0.
    fn at(&self, time: f32) -> f32;
}

pub mod prelude {
    pub use super::adsr::*;
    pub use super::segment::*;
}
