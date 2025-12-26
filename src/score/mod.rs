// Private modules
pub mod compiled_score;
pub mod instances;
pub mod measure;
pub mod staff;

// Public modules and re-exports
pub mod notes;
pub mod score;

pub mod score_builder;

// Re-export essential types directly from the score module
pub mod prelude {
    pub use super::instances::StaffInstance;
    pub use super::measure::{Chord, ChordModifier, Measure};
    pub use super::notes::{DurationModifier, Note, NoteDuration, NoteModifier, NoteName};
    pub use super::score::{Score, TimeSignature};
    pub use super::staff::Staff;
    pub use super::score_builder;
}
