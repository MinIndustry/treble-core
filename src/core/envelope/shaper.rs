//! This trait represents a curve that can be used to shape some other values over time.
//! It can be implemented into finite or infinite shapes, such as envelopes or LFOs.
//! e.g.:
//! * A tremolo effect on the amplitude of a sound can be implemented as a sine wave (infinite shaper)
//! * A vibrato effect on the frequency of a sound can be implemented as a sine wave (infinite shaper)
//! * A drum pitch envelope can be implemented as a decay curve (finite shaper)
pub trait Shaper {
    pub fn get_at(time: f32) -> f32;
}
