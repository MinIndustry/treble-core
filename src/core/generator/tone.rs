use crate::core::envelope::Envelope;

mod blank;
mod sawtooth;
mod sinewave;
mod squarewave;
mod white_noise;

pub use blank::Blank;
pub use sawtooth::SawTooth;
pub use sinewave::SineWave;
pub use squarewave::SquareWave;
pub use white_noise::WhiteNoise;

pub struct ToneGenerator {
    waveform: super::prelude::Waveform,
    frequency_relation: Option<super::prelude::FrequencyRelation>,
    pitch_envelope: Option<Box<dyn Envelope>>,
    amplitude_envelope: Option<Box<dyn Envelope>>,
    phase: f32,
    normalized_time: f32,
    is_stopped: bool,
    current_frequency: f32,
}