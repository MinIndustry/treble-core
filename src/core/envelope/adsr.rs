//! # The ADSR Envelope module.
//! 
//! This module contains the implementaion of the ADSR envelope.
//! 
//! An ADSR envelope is a common type of envelope constisting of four segments
//! 
//! ## Attack
//! 
//! The attack segment represents the time it takes for a note, once triggered, to reach its maximum amplitude.
//! 
//! ## Decay
//! 
//! Once the note has reached its maximum amplitude, the decay segment represents the time it takes to transition from the maximum amplitude to the sustain level.
//! 
//! ## Sustain
//! 
//! The sustain segment represents the constant amplitude during the duration of the note.
//! 
//! ## Release
//! 
//! The release segment represents the time it takes to transition from the sustain level to 0 amplitude once the note is released.
//! 
//! # Representation
//! 
//! The ADSR envelope is represented using four `Segment` structs, each representing one of the segments of the envelope.
//! These segments can be linear, bezier interpolations or any other function implemented in the `Segment` struct.

use std::{default::Default, fmt};

use super::{segment::{Segment, SustainSegment}, Envelope};

#[derive(Debug, Clone)]
pub struct ADSREnvelope {
    pub attack: Box<dyn Segment>,
    pub decay: Box<dyn Segment>,
    pub sustain: Box<dyn SustainSegment>,
    pub release: Box<dyn Segment>,
}

impl Default for ADSREnvelope {
    fn default() -> Self {
        super::adsr_builder::ADSREnvelopeBuilder::default().build()
    }
}

impl ADSREnvelope {
    /// Creates a new Envelope with
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the sustain value of the envelope
    pub fn sustain(&self) -> f32 {
        self.decay.at(1.0)
    }
}

impl fmt::Display for ADSREnvelope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ADSR: Attack: {}, Decay: {}, Sustain: {}, Release: {}",
            self.attack,
            self.decay,
            self.sustain,
            self.release
        )
    }
}

impl Envelope for ADSREnvelope {
    fn at(&self, time: f32, note_off: f32) -> f32 {
        if self.attack.get_duration() > time {  // Still in attack phase
            self.attack.at(self.attack.map_time(0.0, time))
        } else if self.decay.get_duration() > (time - self.attack.get_duration()) {  // In decay phase
            // Fixed: use decay.map_time() instead of attack.map_time()
            self.decay.at(self.decay.map_time(self.attack.get_duration(), time))
        } else {
            if note_off > 0.0 {  // In release phase
                if self.release.get_duration() > (time - note_off) {  // In release
                    self.release.at(self.release.map_time(note_off, time))
                } else {
                    0.0
                }
            } else {
                // Sustain phase: return the end value of decay (sustain level)
                self.decay.at(1.0)
            }
        }
    }

    fn completed(&self, time: f32, note_off: f32) -> bool {
        note_off > 0.0 && self.release.map_time(note_off, time) >= 1.0
    }
}
