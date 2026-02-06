use serde::{Deserialize, Serialize};
use std::fmt;

use crate::core::envelope::Envelope;

/// A constant segment for the sustain phase of an envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstantSegment {
    value: f32,
    duration: Option<f32>,
}

impl fmt::Display for ConstantSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(duration) = self.duration {
            write!(
                f,
                "ConstantSegment with value {} and duration {}",
                self.value, duration
            )
        } else {
            write!(f, "ConstantSegment (sustain) with value {}", self.value)
        }
    }
}

#[typetag::serde]
impl super::Segment for ConstantSegment {
    fn at(&self, _time: f32) -> f32 {
        self.value
    }

    fn get_duration(&self) -> f32 {
        self.duration.unwrap_or(f32::INFINITY)
    }
}

impl super::SustainSegment for ConstantSegment {}

impl ConstantSegment {
    pub fn default_sustain() -> Self {
        Self {
            value: 0.8,
            duration: None,
        }
    }

    pub fn new(value: f32, duration: Option<f32>) -> Self {
        Self { value, duration }
    }
}

#[typetag::serde]
impl Envelope for ConstantSegment {
    fn at(&self, time: f32, note_off: f32) -> f32 {
        !(self.completed(time, note_off)) as i32 as f32 * self.value
    }

    fn completed(&self, time: f32, note_off: f32) -> bool {
        if let Some(duration) = self.duration {
            (time - note_off) < duration
        } else {
            false
        }
    }
}
