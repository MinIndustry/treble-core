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

mod composite;
mod composite_builder;
mod tone;
mod tone_builder;

pub mod prelude {
    use serde::{Deserialize, Serialize};

    pub use super::composite::MultiToneGenerator;
    pub use super::tone::SingleToneGenerator;

    pub mod builder {
        pub use super::super::composite_builder::MultiToneGeneratorBuilder;
        pub use super::super::tone_builder::ToneGeneratorBuilder;
    }

    /// A mixing mode for combining multiple tone generators in the
    /// `CompositeGenerator`.
    /// - Sum: Adds the outputs of all tone generators together.
    /// - Multiply: Multiplies the outputs of all tone generators together.
    /// - Max: Takes the maximum output value from all tone generators.
    /// - Average: Averages the outputs of all tone generators.
    #[derive(Default, Debug, Clone, Serialize, Deserialize)]
    pub enum MixMode {
        #[default]
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
    #[derive(Default, Debug, Clone, Serialize, Deserialize)]
    pub enum Waveform {
        #[default]
        Sine,
        Square,
        Sawtooth,
        Triangle,
        WhiteNoise,
        PinkNoise,
        Blank,
        Err(String), // Only used to inform of a conversion error
    }

    impl Waveform {
        /// All concrete (non-error) waveform variants that can be presented to the user.
        pub fn all() -> Vec<Waveform> {
            vec![
                Waveform::Sine,
                Waveform::Square,
                Waveform::Sawtooth,
                Waveform::Triangle,
                Waveform::WhiteNoise,
                Waveform::PinkNoise,
                Waveform::Blank,
            ]
        }

        /// Machine-readable identifier sent by the frontend when creating a source node.
        pub fn type_id(&self) -> &'static str {
            match self {
                Waveform::Sine => "sine",
                Waveform::Square => "square",
                Waveform::Sawtooth => "saw",
                Waveform::Triangle => "triangle",
                Waveform::WhiteNoise => "whitenoise",
                Waveform::PinkNoise => "pinknoise",
                Waveform::Blank => "blank",
                Waveform::Err(_) => "err",
            }
        }

        /// Human-readable display name shown in the graph node palette.
        pub fn display_name(&self) -> &'static str {
            match self {
                Waveform::Sine => "Sine Wave",
                Waveform::Square => "Square Wave",
                Waveform::Sawtooth => "Sawtooth",
                Waveform::Triangle => "Triangle",
                Waveform::WhiteNoise => "White Noise",
                Waveform::PinkNoise => "Pink Noise",
                Waveform::Blank => "Blank (DC)",
                Waveform::Err(_) => "Unknown",
            }
        }

        pub fn description(&self) -> &'static str {
            match self {
                Waveform::Sine => "Smooth periodic oscillation",
                Waveform::Square => "Alternates between high and low states",
                Waveform::Sawtooth => "Rises linearly then drops sharply",
                Waveform::Triangle => "Rises and falls linearly",
                Waveform::WhiteNoise => "Random signal, equal intensity per frequency",
                Waveform::PinkNoise => "Random signal, equal energy per octave",
                Waveform::Blank => "Constant DC output",
                Waveform::Err(_) => "",
            }
        }

        /// Whether this waveform has a meaningful frequency parameter.
        pub fn has_frequency(&self) -> bool {
            !matches!(
                self,
                Waveform::WhiteNoise | Waveform::PinkNoise | Waveform::Blank | Waveform::Err(_)
            )
        }
    }

    impl From<&str> for Waveform {
        fn from(value: &str) -> Self {
            match value {
                "sine" => Self::Sine,
                "square" => Self::Square,
                "saw" => Self::Sawtooth,
                "triangle" => Self::Triangle,
                "whitenoise" => Self::WhiteNoise,
                "pinknoise" => Self::PinkNoise,
                "blank" => Self::Blank,
                other => {
                    log::warn!("Unknown from for waveform {other}, defaulting to blank");
                    Self::Err(other.into())
                }
            }
        }
    }

    impl From<String> for Waveform {
        fn from(value: String) -> Self {
            match value.as_str() {
                "sine" => Self::Sine,
                "square" => Self::Square,
                "saw" => Self::Sawtooth,
                "triangle" => Self::Triangle,
                "whitenoise" => Self::WhiteNoise,
                "pinknoise" => Self::PinkNoise,
                "blank" => Self::Blank,
                other => {
                    log::warn!("Unknown from for waveform {other}, defaulting to blank");
                    Self::Err(other.into())
                }
            }
        }
    }

    impl From<Waveform> for String {
        fn from(value: Waveform) -> Self {
            match value {
                Waveform::Sine => "sine".into(),
                Waveform::Square => "square".into(),
                Waveform::Sawtooth => "saw".into(),
                Waveform::Triangle => "triangle".into(),
                Waveform::WhiteNoise => "white".into(),
                Waveform::PinkNoise => "pink".into(),
                Waveform::Blank => "blank".into(),
                Waveform::Err(name) => name,
            }
        }
    }

    /// A frequency relation type for tone generation.
    /// - Identity: The frequency is the same as the base frequency.
    /// - Constant(f32): A fixed frequency value.
    /// - Harmonic(u8): A frequency that is a harmonic multiple of a base frequency
    /// - Ratio(f32): A frequency that is a ratio of a base frequency.
    /// - Offset(f32): A frequency that is an offset from a base frequency.
    /// - Semitones(i32): A frequency that is a number of semitones
    #[derive(Default, Debug, Clone, Serialize, Deserialize)]
    pub enum FrequencyRelation {
        #[default]
        Identity,
        Constant(f32),
        Harmonic(u8),
        Ratio(f32),
        Offset(f32),
        Semitones(i32),
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
