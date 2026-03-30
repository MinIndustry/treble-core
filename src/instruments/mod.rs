use crate::Note;
use crate::core::graph::System;

mod custom;
mod drum;
mod keyboard;
mod voices;

pub mod prelude {
    pub use super::custom::*;
    pub use super::drum::*;
    pub use super::keyboard::*;
    pub use super::voices::*;
}

pub trait Instrument: std::fmt::Debug + Send + Sync {
    /// Starts playing the given note
    fn start_note(&mut self, note: Note, velocity: f32);

    /// Stops playing the given note
    fn stop_note(&mut self, note: Note);

    /// Returns the current output of the instrument
    fn get_output(&mut self) -> f32;

    /// Advances the instrument by one tick
    fn tick(&mut self);

    /// Converts this instrument into a self-contained `System` sub-graph.
    /// Used by `AudioGraph::compile()` to assemble all instruments into a
    /// single unified graph for the render thread.
    fn into_system(self: Box<Self>, sample_rate: f32) -> System;
}
