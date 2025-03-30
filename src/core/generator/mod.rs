use crate::core::envelope::prelude::*;
use crate::core::envelope::Envelope;

/// The different types of generator shapes.
pub enum GENERATORS {
    SINE,
    SAW,
    SQUARE,
    NOISE,
    NULL,
}

pub enum FrequencyTransition {
    DIRECT, // Immediate switch (useful for polyphonic instruments with limited generators)
    ENVELOPE(Box<dyn Envelope>),
    LINEAR(f32), // Linear transition of the given duration
}

/// A trait that implements a tone generator.
pub trait ToneGenerator: std::fmt::Debug {
    /// Ticks the generator and returns the current amplitude.
    /// The amplitude is in the range of -1.0 to 1.0.
    fn tick(&mut self, elapsed_time: f32) -> f32;
}

/// Allows an generator to bend its frequency following an envelope
pub trait Bendable {
    fn set_pitch_bend(&mut self, pitch: f32);
}

/// Allows a generator to change its frequency
pub trait VariableFrequency {
    fn change_frequency(&mut self, frequency: f32, transistion: FrequencyTransition);
}

pub trait BendableGenerator: ToneGenerator + Bendable {}
pub trait VariableGenerator: ToneGenerator + VariableFrequency {}
pub trait VariableBendableGenerator: ToneGenerator + VariableFrequency + Bendable {}

#[derive(Debug)]
pub struct Generator {
    pub envelope: ADSREnvelope, // An envelope for the note amplitude
    pitch_curve: Segment,       // An evelope for the note pitch
    tone_generator: Box<dyn ToneGenerator>,
    pub last: (bool, bool, f32), // note on ? - note off ? - last_value
}

impl Generator {
    pub fn new(envelope: ADSREnvelope, tone_generator: Box<dyn ToneGenerator>) -> Generator {
        Self {
            envelope,
            pitch_curve: Segment::default(), // Segment default is a constant segment
            tone_generator,
            last: (false, false, 0.0),
        }
    }

    /// Returns the note value at a point in time, given the note_on, note_off and current time.
    pub fn tick(
        &mut self,
        sample: i32,
        sample_rate: i32,
        note_on_time: f32,
        note_off_time: f32,
    ) -> f32 {
        let time = sample as f32 / sample_rate as f32;

        let ampl = if note_on_time <= time {
            let on_elapsed = time - note_on_time;
            if note_off_time <= time {
                if !self.last.1 {
                    // The note was just released
                    if on_elapsed < self.envelope.decay.end() {
                        // We haven't finished the decay cycle
                        // println!("Note off before end of decay ! - Last value: {}", self.last.2);
                        self.envelope.release.change_from(self.last.2);
                    }
                }
                let off_elapsed = time - note_off_time;
                if off_elapsed < self.envelope.release.end() {
                    self.envelope.release.at(off_elapsed)
                } else {
                    0.0
                }
            } else if on_elapsed < self.envelope.attack.end() {
                self.envelope.attack.at(on_elapsed)
            } else if on_elapsed < self.envelope.decay.end() {
                self.envelope.decay.at(on_elapsed)
            } else {
                self.envelope.sustain()
            }
        } else {
            0.0
        };

        let warp = if time < note_on_time {
            self.pitch_curve.start_value()
        } else if time < note_off_time {
            self.pitch_curve.at(time - note_on_time)
        } else {
            self.pitch_curve.end_value()
        };

        self.last = (note_on_time >= time, note_off_time >= time, ampl);
        ampl * self.tone_generator.tick(1.0 / sample_rate as f32 * warp)
    }

    pub fn set_tone_generator(&mut self, tone_generator: Box<dyn ToneGenerator>) {
        self.tone_generator = tone_generator;
    }

    pub fn set_pitch_bend(&mut self, pitch_curve: Segment) {
        self.pitch_curve = pitch_curve
    }

    pub fn set_ampl_envelope(&mut self, ampl_envelope: ADSREnvelope) {
        self.envelope = ampl_envelope
    }

    pub fn covers(&self, time: f32, note_on_time: f32, note_off_time: f32) -> bool {
        time >= note_on_time && time <= note_off_time + self.envelope.release.end()
    }
}

mod null_generator;
mod saw_tooth;
mod sine_wave;
mod square_wave;
mod white_noise;

pub mod prelude {
    pub use super::null_generator::NullGenerator;
    pub use super::saw_tooth::SawTooth;
    pub use super::sine_wave::SineWave;
    pub use super::square_wave::SquareWave;
    pub use super::white_noise::WhiteNoise;
    pub use super::{ToneGenerator, GENERATORS};
}
