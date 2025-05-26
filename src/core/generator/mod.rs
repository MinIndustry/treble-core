use crate::core::envelope::prelude::*;
use crate::core::envelope::Envelope;
use crate::KeyboardGenerator;

/// The different types of generator shapes.
pub enum GENERATORS {
    SINE,
    SAW,
    SQUARE,
    NOISE,
    NULL,
}

/// Defines the available type of frequency transitions
pub enum FrequencyTransition {
    /// Immediate switch (useful for polyphonic instruments with limited generators)
    DIRECT,
    /// Smooth transition following an envelope
    ENVELOPE(Box<dyn Envelope>),
    /// Simple linear transition accross the given duration
    LINEAR(f32),
}

/// A trait that implements a tone generator. This is a simple generator with no envelope
pub trait ToneGenerator: std::fmt::Debug {
    /// Ticks the generator and returns the current amplitude.
    /// The amplitude is in the range of -1.0 to 1.0.
    fn tick(&mut self, elapsed_time: f32) -> f32;
}

/// The generator trait represents a complete generator,
/// It contains a ToneGenerator & an Envelope
pub trait Generator: std::fmt::Debug + Sync + Send {
    /// Starts the generator. Resets the envelope timer
    /// and start playing the note
    fn start(&mut self);

    /// Stops playing the note. This simply means that the
    /// Envelope will enter its `Release` phase, not that the
    /// note is necessarily over yet.
    fn stop(&mut self);

    /// Ticks the generator of `elapsed_time` seconds.
    fn tick(&mut self, elapsed_time: f32) -> f32;

    /// Whether the generator's envelope has finished its
    /// release phase.
    fn completed(&self) -> bool;

    fn set_frequency(&mut self, frequency: f32);
}

/// A Generator with an envelope shaping its amplitude
pub trait EnvelopedGenerator: Generator {
    fn set_envelope(&mut self, envelope: Box<dyn Envelope + Send + Sync>);
}

/// Allows an generator to bend its frequency following an envelope
pub trait Bendable: Generator {
    fn set_pitch_bend(&mut self, pitch: f32);
}

/// Allows a generator to change its frequency
pub trait VariableFrequency: ToneGenerator {
    fn change_frequency(&mut self, frequency: f32, transistion: FrequencyTransition);
}

/// Allows a generator to change its pitch
pub trait BendableGenerator: Generator + Bendable {}

/// Allows a generator to change its frequency
pub trait VariableToneGenerator: ToneGenerator + VariableFrequency + Send + Sync {}

/// Allows a generator to change its frequency and pitch
pub trait VariableBendableGenerator: ToneGenerator + Bendable {}

mod constant_generator;
mod multisource_generator;
mod null_generator;
mod saw_tooth;
mod simple_generator;
mod sine_wave;
mod square_wave;
mod white_noise;

pub mod prelude {
    pub use super::constant_generator::ConstantGenerator;
    pub use super::multisource_generator::MultiSourceGenerator;
    pub use super::null_generator::NullGenerator;
    pub use super::saw_tooth::SawTooth;
    pub use super::simple_generator::SimpleGenerator;
    pub use super::sine_wave::SineWave;
    pub use super::square_wave::SquareWave;
    pub use super::white_noise::WhiteNoise;
    pub use super::{ToneGenerator, GENERATORS};
}
