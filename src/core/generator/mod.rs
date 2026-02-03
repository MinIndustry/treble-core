//! Generators — oscillators and tone combiners
//!
//! ## Overview
//! Generators produce audio samples over time. Common single-tone generators
//! produce waveforms like sine, square or sawtooth where a single sample at time
//! t can be expressed (for a sine) as: $y(t) = A \sin(2\pi f t + \phi)$ where
//! `A` is amplitude, `f` frequency in Hz and `\phi` phase.
//!
//! The module provides both single-tone generators and composite generators
//! that mix or combine multiple tones. Frequency relations (identity,
//! harmonic, ratio, semitone offsets) allow building harmonic partials easily.
//!
//! ## Implementation notes
//! - `SingleToneGenerator` exposes fine-grained control of frequency and phase.
//! - `MultiToneGenerator` and `CompositeGenerator` provide mixing strategies
//!   (`MixMode`) to sum, multiply or average multiple tone sources.
//! - Be mindful of Nyquist (sample_rate/2) when composing high-frequency
//!   content; aliasing can occur without bandlimiting.

use std::fmt;

mod tone;
mod tone_builder;
mod composite;
mod composite_builder;

use crate::core::generator::prelude::Waveform;

/// The generator trait.
/// An abstract trait for all sound generators.
pub trait Generator: fmt::Debug + Send + Sync {
    fn start(&mut self);
    fn stop(&mut self);
    fn tick(&mut self, time_elapsed: f32) -> f32;
    fn completed(&self) -> bool;
}

/// A generator that produces a single tone.
pub trait SingleToneGenerator: Generator {
    /// Sets the generator's frequency to the given value
    fn set_frequency(&mut self, frequency: f32);

    fn has_frequency_relation(&self) -> bool;

    fn get_waveform(&self) -> &Waveform;

    // Sets the generator's frequency based on its frequency relation (if any)
    fn update_frequency(&mut self, base_frequency: f32);
}

/// A generator that produces multiple tones. Each
/// tone can have its own frequency relation, waveform,
/// and envelopes.
pub trait MultiToneGenerator: Generator {
    fn set_base_frequency(&mut self, frequency: f32);
    fn add_tone(&mut self, tone: tone::ToneGenerator);
    fn tone_count(&self) -> usize;
}

pub mod prelude {
    pub use super::tone::ToneGenerator;
    pub use super::composite::CompositeGenerator;

    pub use super::{MultiToneGenerator, SingleToneGenerator, Generator};

    pub mod builder {
        pub use super::super::composite_builder::CompositeGeneratorBuilder;
        pub use super::super::tone_builder::ToneGeneratorBuilder;
    }

    /// A mixing mode for combining multiple tone generators in the
    /// `CompositeGenerator`.
    /// - Sum: Adds the outputs of all tone generators together.
    /// - Multiply: Multiplies the outputs of all tone generators together.
    /// - Max: Takes the maximum output value from all tone generators.
    /// - Average: Averages the outputs of all tone generators.
    #[derive(Debug)]
    pub enum MixMode {
        Sum,
        Multiply,
        Max,
        Average,
    }

    /// A waveform type for tone generation.
    /// - Sine: A smooth periodic oscillation.
    /// - Square: A waveform that alternates between high and low states.
    /// - Sawtooth: A waveform that rises linearly and then drops sharply.
    /// - Triangle: A waveform that rises and falls linearly.
    /// - WhiteNoise: A random signal with equal intensity at different frequencies.
    /// - PinkNoise: A random signal with equal energy per octave.
    /// - Blank: A constant output defined by amplitude.
    #[derive(Debug)]
    pub enum Waveform {
        Sine,
        Square,
        Sawtooth,
        Triangle,
        WhiteNoise,
        PinkNoise,
        Blank,
    }

    /// A frequency relation type for tone generation.
    /// - Identity: The frequency is the same as the base frequency.
    /// - Constant(f32): A fixed frequency value.
    /// - Harmonic(u8): A frequency that is a harmonic multiple of a base frequency
    /// - Ratio(f32): A frequency that is a ratio of a base frequency.
    /// - Offset(f32): A frequency that is an offset from a base frequency.
    /// - Semitones(i32): A frequency that is a number of semitones
    #[derive(Debug)]
    pub enum FrequencyRelation {
        Identity,
        Constant(f32),
        Harmonic(u8),
        Ratio(f32),
        Offset(f32),
        Semitones(i32)
    }

    impl FrequencyRelation {
        /// Computes the actual frequency based on the base frequency.
        pub fn compute(&self, base_freq: f32) -> f32 {
            match self {
                FrequencyRelation::Identity => base_freq,
                FrequencyRelation::Constant(freq) => *freq,
                FrequencyRelation::Harmonic(harmonic) => base_freq * (*harmonic) as f32,
                FrequencyRelation::Ratio(ratio) => base_freq * ratio,
                FrequencyRelation::Offset(offset) => base_freq + offset,
                FrequencyRelation::Semitones(semitones) => base_freq * 2.0_f32.powi(*semitones),
            }
        }
    }
}
