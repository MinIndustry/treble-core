//! Command and message tests
//!
//! Tests for:
//! - AudioCommand struct creation
//! - AppCommand validation
//! - Velocity validation via App::note_on()
//! - InstrumentAudioMessage field structure
//! - AudioMessage cloning and debug

use rustic::Note;
use rustic::app::commands::{AppCommand, AudioCommand, SystemCommand};
use rustic::audio::AudioMessage;
use rustic::audio::messages::InstrumentAudioMessage;
use rustic::core::utils::NOTES;
use rustic::prelude::App;

// ============================================================================
// AudioCommand struct tests
// ============================================================================

#[test]
fn test_notestart_command_fields() {
    let cmd = AudioCommand::NoteStart {
        instrument_idx: 0,
        note: Note(NOTES::C, 4),
        velocity: 0.5,
    };
    match cmd {
        AudioCommand::NoteStart {
            instrument_idx,
            note,
            velocity,
        } => {
            assert_eq!(instrument_idx, 0);
            assert_eq!(note, Note(NOTES::C, 4));
            assert_eq!(velocity, 0.5);
        }
        _ => panic!("Unexpected variant"),
    }
}

#[test]
fn test_notestop_command_fields() {
    let cmd = AudioCommand::NoteStop {
        instrument_idx: 1,
        note: Note(NOTES::A, 5),
    };
    match cmd {
        AudioCommand::NoteStop {
            instrument_idx,
            note,
        } => {
            assert_eq!(instrument_idx, 1);
            assert_eq!(note, Note(NOTES::A, 5));
        }
        _ => panic!("Unexpected variant"),
    }
}

// ============================================================================
// Velocity validation via App::note_on()
// ============================================================================

#[test]
fn test_notestart_invalid_velocity_negative() {
    let app = App::new();
    let result = app.note_on(0, Note(NOTES::C, 4), -0.1);
    assert!(result.is_err(), "Negative velocity should be rejected");
}

#[test]
fn test_notestart_invalid_velocity_excessive() {
    let app = App::new();
    let result = app.note_on(0, Note(NOTES::C, 4), 1.5);
    assert!(result.is_err(), "Velocity > 1.0 should be rejected");
}

#[test]
fn test_notestart_valid_velocity_boundaries() {
    let app = App::new();
    // 0.0 and 1.0 are valid — expect InvalidInstrumentIndex (no instruments), not a velocity error
    let at_zero = app.note_on(0, Note(NOTES::C, 4), 0.0);
    let at_one = app.note_on(0, Note(NOTES::C, 4), 1.0);
    // Both should fail with InvalidInstrumentIndex, not InvalidParameter
    assert!(
        !format!("{:?}", at_zero).contains("InvalidParameter"),
        "0.0 velocity should be valid"
    );
    assert!(
        !format!("{:?}", at_one).contains("InvalidParameter"),
        "1.0 velocity should be valid"
    );
}

// ============================================================================
// AppCommand validation
// ============================================================================

#[test]
fn test_appcommand_validate() {
    let cmd = AppCommand::System(SystemCommand::Reset);
    assert!(
        cmd.validate().is_ok(),
        "System::Reset should pass validation"
    );
}

// ============================================================================
// InstrumentAudioMessage field structure
// ============================================================================

#[test]
fn test_instrument_message_notestart_fields() {
    let msg = InstrumentAudioMessage::NoteStart {
        source_index: 2,
        note: Note(NOTES::D, 4),
        velocity: 0.7,
    };
    match msg {
        InstrumentAudioMessage::NoteStart {
            source_index,
            note,
            velocity,
        } => {
            assert_eq!(source_index, 2);
            assert_eq!(note, Note(NOTES::D, 4));
            assert_eq!(velocity, 0.7);
        }
        _ => panic!("Unexpected variant"),
    }
}

#[test]
fn test_instrument_message_notestop_fields() {
    let msg = InstrumentAudioMessage::NoteStop {
        source_index: 1,
        note: Note(NOTES::G, 5),
    };
    match msg {
        InstrumentAudioMessage::NoteStop { source_index, note } => {
            assert_eq!(source_index, 1);
            assert_eq!(note, Note(NOTES::G, 5));
        }
        _ => panic!("Unexpected variant"),
    }
}

// ============================================================================
// AudioMessage cloning and debug
// ============================================================================

#[test]
fn test_audiomessage_shutdown() {
    let msg = AudioMessage::Shutdown;
    let cloned = msg.clone();
    assert_eq!(format!("{:?}", msg), format!("{:?}", cloned));
}

#[test]
fn test_audiomessage_clone() {
    let original = AudioMessage::Instrument(InstrumentAudioMessage::NoteStart {
        source_index: 0,
        note: Note(NOTES::A, 4),
        velocity: 0.8,
    });
    let cloned = original.clone();
    assert_eq!(format!("{:?}", original), format!("{:?}", cloned));
}
