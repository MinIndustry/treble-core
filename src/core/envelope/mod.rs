mod adsr;
mod adsr_builder;

mod segment;

/// An envelope that can be used to modulate a signal over time.
/// The base principle is simply to have a function with a varying value over time.
/// This value can then be used to shape either the amplitude, frequency or any other parameter of a sound.
pub trait Envelope: std::fmt::Display + std::fmt::Debug + Send + Sync {
    /// Returns the envelope value at the given point in time. The timestamps
    /// is expected to be mapped to the envelope's duration, that is the
    /// minimum value is 0.0.
    fn at(&self, time: f32, note_off: f32) -> f32;

    /// Returns whether the envelope has completed or not based on the
    /// current time & note_off timestamp
    fn completed(&self, time: f32, note_off: f32) -> bool;
}

pub mod prelude {
    pub use super::adsr::*;
    pub use super::adsr_builder::*;
    pub use super::segment::*;
}
