use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
pub enum NoteDuration {
    Large,     // Octuple whole note
    Long,      // Quadruple whole note
    Breve,     // Double whole note
    SemiBreve, // Whole note
    Minim,     // Half note
    #[default]
    Crotchet, // Quarter note
    Quaver,    // Eighth note
    SemiQuaver, // Sixteenth note
    DemiSemiQuaver, // Thirty-second note
    HemiDemiSemiQuaver, // Sixty-fourth note
    SemiHemiDemiSemiQuaver, // One hundred twenty-eighth note
    DemiSemiHemiDemiSemiQuaver, // Two hundred fifty-sixth note
    Tuplet(u8), // Usually 3
}

impl NoteDuration {
    pub fn duration(&self) -> usize {
        match self {
            Self::DemiSemiHemiDemiSemiQuaver => 1,
            Self::SemiHemiDemiSemiQuaver => 2,
            Self::HemiDemiSemiQuaver => 4,
            Self::DemiSemiQuaver => 8,
            Self::SemiQuaver => 16,
            Self::Quaver => 32,
            Self::Crotchet | Self::Tuplet(_) => 64,
            Self::Minim => 128,
            Self::SemiBreve => 256,
            Self::Breve => 512,
            Self::Long => 1024,
            Self::Large => 2048,
        }
    }
}

#[derive(Default, Serialize, Deserialize, Debug)]
pub enum DurationModifier {
    #[default]
    None,
    Dotted,
    DoubleDotted,
}

#[derive(Default, Serialize, Deserialize, Debug)]
pub enum NoteModifier {
    Flat,
    DoubleFlat,
    Sharp,
    DoubleSharp,
    Natural,
    #[default]
    None,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub enum NoteName {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    #[default]
    Pause,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Note {
    pub duration: NoteDuration,
    pub duration_modifier: DurationModifier,
    pub note: NoteName,
    pub modifier: NoteModifier,
    pub octave: u8,
    pub tied: bool, // Whether the note continues with its next iteration
}

impl Note {
    pub fn new_pause(duration: NoteDuration) -> Result<Self, String> {
        Ok(Self {
            duration,
            duration_modifier: DurationModifier::None,
            note: NoteName::Pause, // 0 Means no note
            modifier: NoteModifier::None,
            octave: 0,
            tied: false,
        })
    }

    pub fn new(
        duration: NoteDuration,
        duration_modifier: DurationModifier,
        note: NoteName,
        modifier: NoteModifier,
        octave: u8,
        tied: bool,
    ) -> Result<Self, String> {
        Ok(Self {
            duration,
            duration_modifier,
            note,
            modifier,
            octave,
            tied,
        })
    }

    pub fn duration(&self) -> usize {
        match self.duration_modifier {
            DurationModifier::None => self.duration.duration(),
            DurationModifier::Dotted => self.duration.duration() + self.duration.duration() / 2,
            DurationModifier::DoubleDotted => {
                self.duration.duration()
                    + self.duration.duration() / 2
                    + self.duration.duration() / 4
            }
        }
    }
}
