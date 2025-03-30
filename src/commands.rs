// An event that represents a "command" that the user can input.
// E.g. NOTEDOWN means a note should start playing. NOTEUP means a note should stop playing.
pub enum Event {
    NOTE_DOWN,
    NOTE_UP,
}

// Modifier for the above events
pub enum Modifier {
    OCTAVE,  // Modifier for the octave. Used to differenciate between the two keyboard lines of notes.
}


pub struct Command {
    pub event: Event,
    pub modifier: Modifier,
    pub value: u16,
}
