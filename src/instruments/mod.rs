use crate::Note;

/// Drum related modules
mod drum;

/// Keyboard related modules
mod keyboard;

pub mod prelude {
    pub use super::drum::*;
    pub use super::keyboard::*;
}

/// The `Instrument` trait unifies different instruments structure,
/// allowing to play complex graphs using a simple interface.
pub trait Instrument {
    /// Starts playing the given note
    fn start_note(&mut self, note: Note, velocity: f32);

    /// Stops playing the given note
    fn stop_note(&mut self, note: Note);

    /// Returns the current output of the instrument
    fn get_output(&mut self) -> f32;

    /// Advances the instrument by one tick
    fn tick(&mut self);
}
