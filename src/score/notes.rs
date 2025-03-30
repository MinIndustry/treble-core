use serde::{Deserialize, Serialize};

use crate::core::tones::NOTES;

#[derive(Serialize, Deserialize, Debug)]
pub enum NoteDuration {
    Silence,          // 1 time silence
    HalfPause,        // 2 times
    Pause,            // 4 times
    Whole,            // 4 times
    Half,             // 2 times
    Quarter,          // 1 time
    Eighth,           // half time
    Sixteenth,        // Quarter time
    ThirthySencondth, // Eighth time
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Note {
    pub duration: NoteDuration,
    pub note: NOTES,
    pub octave: u8,
    pub bound: bool, // Whether the note continues with its next iteration
}

impl Note {
    pub fn new_pause(duration: usize) -> Result<Self, String> {
        Ok(Self {
            duration: match duration {
                1 => Ok(NoteDuration::Silence),
                2 => Ok(NoteDuration::HalfPause),
                4 => Ok(NoteDuration::Pause),
                _ => Err(format!("Invalid duration for a pause: {duration}")),
            }?,
            note: NOTES::A,
            octave: 0,
            bound: false,
        })
    }
}
