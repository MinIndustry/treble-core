//! Simple note representation for audio processing
//!
//! This module provides a lightweight note type that combines a musical note
//! name with an octave number, suitable for audio processing and instrument control.

use super::tones::NOTES;
use serde::{Deserialize, Serialize};

/// A simple musical note with its octave
///
/// This is a lightweight representation of a musical note consisting of:
/// - A note name from the chromatic scale (C, C#, D, etc.)
/// - An octave number (typically 0-8)
///
/// # Examples
///
/// ```
/// use rustic::core::utils::{Note, NOTES};
///
/// let middle_c = Note(NOTES::C, 4);
/// let a440 = Note(NOTES::A, 4);
/// ```
#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Note(pub NOTES, pub u8);

impl Note {
    /// Create a new note from a note name and octave
    pub fn new(note: NOTES, octave: u8) -> Self {
        Self(note, octave)
    }

    /// Get the note name
    pub fn note(&self) -> NOTES {
        self.0
    }

    /// Get the octave
    pub fn octave(&self) -> u8 {
        self.1
    }

    /// Calculate the frequency of this note in Hz
    ///
    /// Uses equal temperament tuning with A4 = 440 Hz
    pub fn frequency(&self) -> f32 {
        use super::tones::TONES_FREQ;

        let octave = self.1 as usize;
        let note_index = self.0 as usize;

        if octave < TONES_FREQ[0].len() {
            TONES_FREQ[note_index][octave]
        } else {
            // For octaves beyond our table, use the formula
            let base_freq = TONES_FREQ[note_index][0];
            base_freq * (2.0_f32).powi(octave as i32)
        }
    }

    /// Convert from MIDI note number
    ///
    /// MIDI note 60 = C4 (middle C)
    pub fn from_midi(midi_note: u8) -> Self {
        let octave = midi_note / 12;
        let note_index = midi_note % 12;
        let note = NOTES::from(note_index);
        Self(note, octave.saturating_sub(1)) // MIDI octave -1 adjustment
    }

    /// Convert to MIDI note number
    ///
    /// Returns the MIDI note number (0-127) for this note
    pub fn to_midi(&self) -> u8 {
        let octave_offset = (self.1 + 1) * 12; // MIDI octave +1 adjustment
        let note_offset = self.0 as u8;
        (octave_offset + note_offset).min(127)
    }

    /// Transpose the note by a number of semitones
    ///
    /// Positive values transpose up, negative values transpose down
    pub fn transpose(self, semitones: i8) -> Self {
        let midi = self.to_midi() as i8;
        let new_midi = (midi + semitones).clamp(0, 127) as u8;
        Self::from_midi(new_midi)
    }
}

impl std::fmt::Display for Note {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let note_name = match self.0 {
            NOTES::C => "C",
            NOTES::CS => "C#",
            NOTES::D => "D",
            NOTES::DS => "D#",
            NOTES::E => "E",
            NOTES::F => "F",
            NOTES::FS => "F#",
            NOTES::G => "G",
            NOTES::GS => "G#",
            NOTES::A => "A",
            NOTES::AS => "A#",
            NOTES::B => "B",
        };
        write!(f, "{}{}", note_name, self.1)
    }
}

// Note: unit tests for `Note` have been moved to `src/tests/notes.rs` to
// centralize the test suite. See that file for coverage and additional cases.
