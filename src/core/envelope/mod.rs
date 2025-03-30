mod envelope;
mod segment;
mod bezier;

/// A shape that can be used to modulate a signal over time.
pub trait Shape: std::fmt::Display {
    /// Returns the shaper values at the given point in time
    fn get_at(&self, time: f32) -> f32;
}

/// An envelope that can be used to modulate a signal over time.
/// The base principle is simply to have a function with a varying value over time.
/// This value can then be used to shape either the amplitude, frequency or any other parameter of a sound.
pub trait Envelope: std::fmt::Display {
    /// Returns the envelope value at the given point in time
    fn at(&self, time: f32) -> f32;
}

pub mod prelude {
    pub use super::envelope::*;
    pub use super::segment::*;
    pub use super::bezier::*;
}
