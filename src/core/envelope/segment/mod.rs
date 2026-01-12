use dyn_clone::DynClone;
use std::fmt;

mod bezier;
mod constant;
mod function;
mod linear;

pub use bezier::BezierSegment;
pub use constant::ConstantSegment;
pub use function::FunctionSegment;
pub use linear::LinearSegment;

/// A segment is a part of an envelope that maps a time range to amplitude values.
/// The time range is normalized between 0.0 and 1.0 for the duration of the segment.
///
/// It is up to the envelope implementation to manage the time mapping and sequencing of segments.
/// For convenience, the segment trait provides a method to map the global time to the segment's local time.
pub trait Segment: fmt::Display + DynClone + fmt::Debug + Send + Sync {
    /// Returns the amplitude at the given time position within the segment.
    /// Any value of time above one will be clamped to the end value of the segment.
    /// (time > 1.0 ? 1.0 : time)
    fn at(&self, time: f32) -> f32;

    /// Returns the duration in seconds of the segment.
    fn get_duration(&self) -> f32;

    /// Maps the global time to the segment's local time [0.0 - 1.0]
    /// Any returned value above 1.0 means that the segment has completed.
    fn map_time(&self, segment_start: f32, current_time: f32) -> f32 {
        (current_time - segment_start) / self.get_duration()
    }
}

/// A sustain segment is a special kind of segment that does not define a time range,
/// but can be queried for as long as the note is held. It can be a constant
/// value or any other function in R+.
pub trait SustainSegment: Segment {
    fn get_duration(&self) -> f32 {
        f32::INFINITY
    }
}

dyn_clone::clone_trait_object!(Segment);
dyn_clone::clone_trait_object!(SustainSegment);
