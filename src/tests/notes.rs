use pretty_assertions::assert_eq;

use crate::core::utils::{Note, NOTES};

#[test]
fn test_note_creation() {
    let note = Note::new(NOTES::A, 4);
    assert_eq!(note.note(), NOTES::A);
    assert_eq!(note.octave(), 4);
}

#[test]
fn test_note_frequency() {
    let a4 = Note::new(NOTES::A, 4);
    let frequency = a4.frequency();

    // A4 should be 440 Hz
    assert!(
        (frequency - 440.0).abs() < 0.1,
        "A4 frequency should be close to 440Hz, got {}",
        frequency
    );
}

#[test]
fn test_note_midi_conversion() {
    // Middle C (C4) is MIDI note 60
    let c4 = Note::from_midi(60);
    assert_eq!(c4.note(), NOTES::C);
    assert_eq!(c4.octave(), 4);
    assert_eq!(c4.to_midi(), 60);

    // A4 (440Hz) is MIDI note 69
    let a4 = Note::from_midi(69);
    assert_eq!(a4.note(), NOTES::A);
    assert_eq!(a4.octave(), 4);
    assert_eq!(a4.to_midi(), 69);
}

#[test]
fn test_note_transpose() {
    let c4 = Note::new(NOTES::C, 4);

    // Transpose up by 2 semitones should give D4
    let d4 = c4.transpose(2);
    assert_eq!(d4.note(), NOTES::D);
    assert_eq!(d4.octave(), 4);

    // Transpose up by 12 semitones should give C5
    let c5 = c4.transpose(12);
    assert_eq!(c5.note(), NOTES::C);
    assert_eq!(c5.octave(), 5);

    // Transpose down by 1 semitone should give B3
    let b3 = c4.transpose(-1);
    assert_eq!(b3.note(), NOTES::B);
    assert_eq!(b3.octave(), 3);
}

#[test]
fn test_note_display() {
    let c_sharp_4 = Note::new(NOTES::CS, 4);
    assert_eq!(format!("{}", c_sharp_4), "C#4");

    let f_sharp_3 = Note::new(NOTES::FS, 3);
    assert_eq!(format!("{}", f_sharp_3), "F#3");

    let b5 = Note::new(NOTES::B, 5);
    assert_eq!(format!("{}", b5), "B5");
}

#[test]
fn test_note_equality() {
    let note1 = Note::new(NOTES::A, 4);
    let note2 = Note::new(NOTES::A, 4);
    let note3 = Note::new(NOTES::B, 4);

    assert_eq!(note1, note2);
    assert_ne!(note1, note3);
}
