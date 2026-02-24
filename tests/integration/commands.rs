//! Command Validation and Translation Tests
//!
//! This test suite validates the command validation and translation system:
//! - Command validation for valid inputs
//! - Command validation failure for invalid inputs
//! - Translation from AudioCommand to AudioMessage
//! - Error types and messages

use rustic::Note;
use rustic::app::commands::{AppCommand, AudioCommand, SystemCommand};
use rustic::audio::messages::InstrumentAudioMessage;
use rustic::audio::{AudioMessage, CommandError};
use rustic::core::utils::NOTES;

// ============================================================================
// Command Validation Tests - Valid Commands
// ============================================================================

#[test]
fn test_notestart_valid_parameters() {
    let cmd = AudioCommand::NoteStart {
        instrument_idx: 0,
        note: Note(NOTES::C, 4),
        velocity: 0.5,
    };
    assert!(
        cmd.into_audio_message().is_ok(),
        "NoteStart with valid parameters should pass"
    );

    let cmd = AudioCommand::NoteStart {
        instrument_idx: 1,
        note: Note(NOTES::A, 3),
        velocity: 1.0,
    };
    assert!(
        cmd.into_audio_message().is_ok(),
        "NoteStart with max velocity should pass"
    );

    let cmd = AudioCommand::NoteStart {
        instrument_idx: 0,
        note: Note(NOTES::G, 5),
        velocity: 0.0,
    };
    assert!(
        cmd.into_audio_message().is_ok(),
        "NoteStart with zero velocity should pass"
    );
}

#[test]
fn test_notestop_valid_parameters() {
    let cmd = AudioCommand::NoteStop {
        instrument_idx: 0,
        note: Note(NOTES::C, 4),
    };
    assert!(
        cmd.into_audio_message().is_ok(),
        "NoteStop with valid parameters should pass"
    );

    let cmd = AudioCommand::NoteStop {
        instrument_idx: 1,
        note: Note(NOTES::A, 5),
    };
    assert!(
        cmd.into_audio_message().is_ok(),
        "NoteStop with different instrument should pass"
    );
}

#[test]
fn test_commands_without_validation() {
    // AudioCommand::Shutdown should translate successfully
    let cmd = AudioCommand::Shutdown;
    assert!(
        cmd.into_audio_message().is_ok(),
        "Shutdown should translate"
    );

    // App commands that always pass validation
    let app_commands: Vec<AppCommand> = vec![AppCommand::System(SystemCommand::Reset)];

    for cmd in app_commands {
        assert!(
            cmd.validate().is_ok(),
            "AppCommand {:?} should pass validation",
            cmd
        );
    }
}

// ============================================================================
// Command Validation Tests - Invalid Commands
// ============================================================================

#[test]
fn test_notestart_invalid_velocity_negative() {
    let cmd = AudioCommand::NoteStart {
        instrument_idx: 0,
        note: Note(NOTES::C, 4),
        velocity: -0.1,
    };
    let result = cmd.into_audio_message();

    assert!(
        result.is_err(),
        "NoteStart with negative velocity should fail"
    );
    match result.unwrap_err() {
        CommandError::InvalidVolume(vel) => {
            assert_eq!(vel, -0.1, "Error should report velocity -0.1");
        }
    }
}

#[test]
fn test_notestart_invalid_velocity_excessive() {
    let cmd = AudioCommand::NoteStart {
        instrument_idx: 0,
        note: Note(NOTES::C, 4),
        velocity: 1.5,
    };
    let result = cmd.into_audio_message();

    assert!(result.is_err(), "NoteStart with velocity > 1.0 should fail");
    match result.unwrap_err() {
        CommandError::InvalidVolume(vel) => {
            assert_eq!(vel, 1.5, "Error should report velocity 1.5");
        }
    }
}

// ============================================================================
// AudioCommand to AudioMessage Translation Tests
// ============================================================================

#[test]
fn test_translate_notestart() {
    let cmd = AudioCommand::NoteStart {
        instrument_idx: 2,
        note: Note(NOTES::D, 4),
        velocity: 0.7,
    };
    let audio_msg = cmd.into_audio_message().unwrap();

    match audio_msg {
        AudioMessage::Instrument(InstrumentAudioMessage::NoteStart {
            instrument_idx,
            note,
            velocity,
        }) => {
            assert_eq!(instrument_idx, 2, "Instrument index should be 2");
            assert_eq!(note, Note(NOTES::D, 4), "Note should be D4");
            assert_eq!(velocity, 0.7, "Velocity should be 0.7");
        }
        _ => panic!("Expected AudioMessage::Instrument(NoteStart)"),
    }
}

#[test]
fn test_translate_notestop() {
    let cmd = AudioCommand::NoteStop {
        instrument_idx: 1,
        note: Note(NOTES::G, 5),
    };
    let audio_msg = cmd.into_audio_message().unwrap();

    match audio_msg {
        AudioMessage::Instrument(InstrumentAudioMessage::NoteStop {
            instrument_idx,
            note,
        }) => {
            assert_eq!(instrument_idx, 1, "Instrument index should be 1");
            assert_eq!(note, Note(NOTES::G, 5), "Note should be G5");
        }
        _ => panic!("Expected AudioMessage::Instrument(NoteStop)"),
    }
}

#[test]
fn test_translate_shutdown() {
    let cmd = AudioCommand::Shutdown;
    let audio_msg = cmd.into_audio_message().unwrap();

    match audio_msg {
        AudioMessage::Shutdown => {}
        _ => panic!("Expected AudioMessage::Shutdown"),
    }
}

// ============================================================================
// Error Display Tests
// ============================================================================

#[test]
fn test_error_display_invalid_volume() {
    let error = CommandError::InvalidVolume(1.5);
    let error_msg = format!("{}", error);

    assert!(error_msg.contains("Invalid volume"));
    assert!(error_msg.contains("1.5"));
    assert!(error_msg.contains("must be 0.0-1.0"));
}

// ============================================================================
// AudioMessage Cloning Tests
// ============================================================================

#[test]
fn test_audiomessage_clone() {
    use rustic::audio::messages::InstrumentAudioMessage;
    use rustic::core::utils::NOTES;

    let original = AudioMessage::Instrument(InstrumentAudioMessage::NoteStart {
        instrument_idx: 0,
        note: Note(NOTES::A, 4),
        velocity: 0.8,
    });

    let cloned = original.clone();

    assert_eq!(
        format!("{:?}", original),
        format!("{:?}", cloned),
        "Cloned AudioMessage should be identical"
    );
}
