use log::info;
use serde::{Deserialize, Serialize};

use super::notes::{Note, NoteDuration};
use super::score::TimeSignature;

#[derive(Serialize, Deserialize, Default)]
pub enum ChordModifier {
    #[default]
    None,
    Arpeggio,
    ArpeggioInverted,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Chord {
    notes: Vec<Note>,
    modifier: ChordModifier,
}

impl Chord {
    pub fn new(notes: Vec<Note>, modifier: ChordModifier) -> Self {
        Self { notes, modifier }
    }

    /// Duration of the chord in term of crotchets
    pub fn duration(&self) -> usize {
        self.notes.iter().map(|n| n.duration()).max().unwrap_or(0)
    }

    pub fn add_note(&mut self, note: Note) {
        self.notes.push(note);
    }

    /// Replaces the current chord's notes with the given note
    pub fn set_note(&mut self, note: Note) {
        self.notes = vec![note];
    }
}

/// A measure contains a given amount of notes
#[derive(Serialize, Deserialize)]
pub struct Measure {
    size: usize, // Number of Crotchet notes
    #[serde(skip)]
    chords: Vec<Chord>,
    chords_set: Vec<(usize, Chord)>, // Sets of notes in the measure (scrambled). Used for serialization
}

impl Measure {
    /// Creates a new measure with no notes.
    pub fn new(signature: &TimeSignature) -> Self {
        Self {
            size: signature.0,
            chords: Vec::from_iter(
                (0..signature.0 * NoteDuration::Crotchet.duration()).map(|_| Chord::default()),
            ),
            chords_set: Vec::new(),
        }
    }

    /// Checks if the measure is full. A measure is full if
    /// the sum of its chords durations is equal to the time
    /// signature's numerator times the duration of a Crotchet
    pub fn is_full(&self) -> bool {
        self.chords.iter().map(|c| c.duration()).sum::<usize>()
            == self.size * NoteDuration::Crotchet.duration()
    }

    /// Returns the index of the next available space for a note in the measure
    pub fn current_index(&self) -> usize {
        self.chords.iter().map(|c| c.duration()).sum::<usize>()
    }

    /// Adds a note in the chord at the given time position.
    pub fn add_note(&mut self, time_index: usize, note: Note) -> Result<(), String> {
        if let Some(index) = self.chords_set.iter().position(|(id, _)| *id == time_index) {
            info!("Adding note to existing chord at time index {}", time_index);
            self.chords_set[index].1.add_note(note);
            Ok(())
        } else {
            info!("Adding new chord at time index {}", time_index);
            self.chords_set.push((time_index, Chord::default()));
            self.chords_set.last_mut().unwrap().1.add_note(note);
            Ok(())
        }
    }

    /// Sets the note at the given time position, overriding the chord currently positioned there.
    pub fn set_note(&mut self, time_index: usize, note: Note) -> Result<(), String> {
        if let Some(index) = self.chords_set.iter().position(|(id, _)| *id == time_index) {
            info!(
                "Setting note in existing chord at time index {}",
                time_index
            );
            self.chords_set[index].1.set_note(note);
            Ok(())
        } else {
            info!("Setting new chord at time index {}", time_index);
            self.chords_set.push((time_index, Chord::default()));
            self.chords_set.last_mut().unwrap().1.set_note(note);
            Ok(())
        }
    }

    /// Sets the chord at the given position
    pub fn set_chord(&mut self, time_index: usize, chord: Chord) -> Result<(), String> {
        if let Some(index) = self.chords_set.iter().position(|(id, _)| *id == time_index) {
            info!("Setting chord at time index {}", time_index);
            self.chords_set[index].1 = chord;
            Ok(())
        } else {
            info!("Setting new chord at time index {}", time_index);
            self.chords_set.push((time_index, chord));
            Ok(())
        }
    }
}
