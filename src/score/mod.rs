// Private modules
mod compiled_score;
mod instances;
mod measure;
mod staff;

// Public modules and re-exports
pub mod notes;
pub mod score;

// Re-export essential types directly from the score module
pub use instances::StaffInstance;
pub use measure::{Chord, ChordModifier, Measure};
pub use notes::{DurationModifier, Note, NoteDuration, NoteModifier, NoteName};
pub use score::{Score, TimeSignature};
pub use staff::Staff;
