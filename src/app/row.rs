use crate::core::tones::NOTES;
use crate::Note;

pub struct Row {
    pub instrument: usize, // Index of the instrument in app's array
    pub octave: u8,        // Current octave of the row
}

impl Default for Row {
    fn default() -> Row {
        Row {
            instrument: 0,
            octave: 4,
        }
    }
}

impl Row {
    pub fn get_note(&mut self, note: u8) -> Note {
        Note(NOTES::from(note), self.octave)
    }
}
