use crate::score::prelude::*;

#[test]
pub fn test_chord_duration() {
    let mut chord = Chord::new(
        vec![Note::new(
            NoteDuration::Crotchet,
            DurationModifier::None,
            NoteName::A,
            NoteModifier::None,
            4,
            false,
        )],
        ChordModifier::None,
    );
    assert_eq!(chord.duration(), 64);

    chord.add_note(Note::new(
        NoteDuration::Crotchet,
        DurationModifier::Dotted,
        NoteName::B,
        NoteModifier::None,
        4,
        false,
    ));

    assert_eq!(chord.duration(), 96);
}
