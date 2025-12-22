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

use super::{segment::{Segment, SustainSegment, LinearSegment, ConstantSegment}, Envelope};
use crate::core::{generator::prelude::SimpleGenerator};
use crate::core::generator::VariableToneGenerator;

#[derive(Debug, Clone)]
pub struct ADSREnvelope {
    pub attack: Box<dyn Segment>,
    pub decay: Box<dyn Segment>,
    pub sustain: Box<dyn SustainSegment>,
    pub release: Box<dyn Segment>,
}

impl Default for ADSREnvelope {
    fn default() -> Self {
        super::adsr_builder::ADSREnvelopeBuider::default().build()
    }
}

impl ADSREnvelope {
    /// Creates a new Envelope with
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a simple constant envelope.
    /// The note reaches the maximum amplitude as soon as it is played and
    /// the minimum as soon as it is released.
    pub fn constant() -> Self {
        super::adsr_builder::ADSREnvelopeBuider::constant().build()
    }

    /// Creates a generator from a tone generator and the envelope (cloned)
    pub fn attach_amplitude(&self, generator: Box<dyn VariableToneGenerator>) -> SimpleGenerator {
        SimpleGenerator::new(Box::from(self.clone()), generator)
    }

    /// Returns the sustain value of the envelope
    pub fn sustain(&self) -> f32 {
        self.decay.end_value()
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
        if self.attack.covers(time) {
            self.attack.at(time)
        } else if self.decay.covers(time) {
            self.decay.at(time)
        } else {
            if note_off > 0.0 {
                if self.release.covers(time - note_off) {
                    self.release.at(time - note_off)
                } else {
                    0.0
                }
            } else {
                self.decay.end_value()
            }
        }
    }

    fn completed(&self, time: f32, note_off: f32) -> bool {
        note_off > 0.0 && time - note_off > self.release.end()
    }
}
