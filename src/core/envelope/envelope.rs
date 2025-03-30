use std::{default::Default, fmt};

use super::{segment::Segment, Envelope};
use crate::core::generator::{Generator, ToneGenerator};

// ADSR
// This is a simple ADSR envelope generator that can be used to control the amplitude of a sound over time.
#[derive(Debug, Clone)]
pub struct ADSREnvelope {
    pub attack: Segment,
    pub decay: Segment,
    pub release: Segment,
}

impl ADSREnvelope {
    /// Creates a new Envelope with
    pub fn new() -> Self {
        Self {
            attack: Segment::default(),
            decay: Segment::default(),
            release: Segment::default(),
        }
    }

    pub fn with_attack(mut self, duration: f32, to: f32, control: Option<(f32, f32)>) -> Self {
        self.set_attack(duration, to, control);
        self
    }

    pub fn with_decay(mut self, duration: f32, to: f32, control: Option<(f32, f32)>) -> Self {
        self.set_decay(duration, to, control);
        self
    }

    pub fn with_release(mut self, duration: f32, to: f32, control: Option<(f32, f32)>) -> Self {
        self.set_release(duration, to, control);
        self
    }

    /// A simple envelope, where the note is a maximum as soon as it is played and
    /// at minimum as soon as it is released
    pub fn constant() -> Self {
        Self {
            attack: Segment::new(0.0, 1.0, 0.0, 0.0, None),
            decay: Segment::new(1.0, 1.0, 0.0, 0.0, None),
            release: Segment::new(1.0, 0.0, 0.0, 0.0, None),
        }
    }

    /// Creates a generator from a tone generator and the envelope (cloned)
    pub fn attach_amplitude(&self, generator: Box<dyn ToneGenerator>) -> Generator {
        Generator::new(self.clone(), generator)
    }

    /// Returns the sustain value of the envelope
    pub fn sustain(&self) -> f32 {
        self.decay.end_value()
    }

    pub fn set_attack(&mut self, duration: f32, to: f32, control: Option<(f32, f32)>) -> bool {
        self.attack = Segment::new(0.0, to, duration, 0.0, control);
        true
    }

    pub fn set_decay(&mut self, duration: f32, to: f32, control: Option<(f32, f32)>) -> bool {
        // TODO: Check decay is correct
        self.decay = Segment::new(
            self.attack.end_value(),
            to,
            duration,
            self.attack.end(),
            control,
        );
        true
    }

    // For release, elapsed is from 0 to release duration
    pub fn set_release(&mut self, duration: f32, to: f32, control: Option<(f32, f32)>) -> bool {
        // TODO: Check release is correct
        self.release = Segment::new(self.decay.end_value(), to, duration, 0.0, control);
        true
    }
}

impl fmt::Display for ADSREnvelope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ADSR: Attack: {}, Decay: {}, Sustain: {}, Release: {}",
            self.attack,
            self.decay,
            self.sustain(),
            self.release
        )
    }
}

impl Envelope for ADSREnvelope {
    fn at(&self, time: f32) -> f32 {
        if self.attack.covers(time) {
            self.attack.at(time)
        } else if self.decay.covers(time) {
            self.decay.at(time)
        } else if self.release.covers(time) {
            self.release.at(time)
        } else {
            0.0
        }
    }
}
