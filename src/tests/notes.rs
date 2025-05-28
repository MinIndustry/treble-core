use pretty_assertions::assert_eq;

use crate::core::note::Note;

#[test]
fn test_note_coverage() {
    let mut note = Note::new(440.0, 1.0, 1.0);

    assert_eq!(
        note.is_completed(1.5),
        false,
        "Note should still be on at 1.5 seconds"
    );

    assert_eq!(
        note.is_completed(0.5),
        false,
        "Note can't be completed before it's started"
    );

    assert_eq!(
        note.is_completed(2.5),
        true,
        "Note should be marked completed after it's been completed"
    );
}

#[test]
fn test_note_plays() {
    let mut note = Note::new(440.0, 1.0, 1.0);
    let sample_rate: i32 = 100;

    let count = (0..100)
        .map(|_| note.tick(sample_rate))
        .filter(|v| *v > 0.5 || *v < 0.5)
        .count();

    assert_ne!(
        count, 0,
        "There should be some values with a difference > 0.5 from zero in a second of sample"
    );
}
