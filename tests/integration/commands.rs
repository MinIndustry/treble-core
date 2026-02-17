//! Command Validation and Translation Tests
//!
//! This test suite validates the command validation and translation system:
//! - Command validation for valid inputs
//! - Command validation failure for invalid inputs
//! - Translation from AudioCommand to AudioMessage
//! - Error types and messages

use rustic::app::commands::{AppCommand, AudioCommand, LiveCommand, SystemCommand};
use rustic::audio::messages::InstrumentAudioMessage;
use rustic::audio::{AudioMessage, CommandError};
use rustic::prelude::App;

// ============================================================================
// Command Validation Tests - Valid Commands
// ============================================================================

#[test]
fn test_notestart_valid_parameters() {
    let app = App::new();

    let cmd = AudioCommand::NoteStart {
        note: 0,
        row: 0,
        velocity: 0.5,
    };
    assert!(
        cmd.into_audio_message(&app).is_ok(),
        "NoteStart with valid parameters should pass"
    );

    let cmd = AudioCommand::NoteStart {
        note: 11,
        row: 1,
        velocity: 1.0,
    };
    assert!(
        cmd.into_audio_message(&app).is_ok(),
        "NoteStart with max velocity should pass"
    );

    let cmd = AudioCommand::NoteStart {
        note: 5,
        row: 0,
        velocity: 0.0,
    };
    assert!(
        cmd.into_audio_message(&app).is_ok(),
        "NoteStart with zero velocity should pass"
    );
}

#[test]
fn test_notestop_valid_parameters() {
    let app = App::new();

    let cmd = AudioCommand::NoteStop { note: 0, row: 0 };
    assert!(
        cmd.into_audio_message(&app).is_ok(),
        "NoteStop with valid row should pass"
    );

    let cmd = AudioCommand::NoteStop { note: 11, row: 1 };
    assert!(
        cmd.into_audio_message(&app).is_ok(),
        "NoteStop with row 1 should pass"
    );
}

#[test]
fn test_setoctave_valid_parameters() {
    let app = App::new();

    for octave in 0..=8 {
        for row in 0..=1 {
            let cmd = AppCommand::Live(LiveCommand::SetOctave { octave, row });
            assert!(
                cmd.validate(&app).is_ok(),
                "SetOctave({}, {}) should pass",
                octave,
                row
            );
        }
    }
}

#[test]
fn test_octaveup_valid_parameters() {
    let app = App::new();

    let cmd = AppCommand::Live(LiveCommand::OctaveUp(0));
    assert!(cmd.validate(&app).is_ok(), "OctaveUp(0) should pass");

    let cmd = AppCommand::Live(LiveCommand::OctaveUp(1));
    assert!(cmd.validate(&app).is_ok(), "OctaveUp(1) should pass");
}

#[test]
fn test_octavedown_valid_parameters() {
    let app = App::new();

    let cmd = AppCommand::Live(LiveCommand::OctaveDown(0));
    assert!(cmd.validate(&app).is_ok(), "OctaveDown(0) should pass");

    let cmd = AppCommand::Live(LiveCommand::OctaveDown(1));
    assert!(cmd.validate(&app).is_ok(), "OctaveDown(1) should pass");
}

#[test]
fn test_commands_without_validation() {
    let app = App::new();

    // AudioCommand::Shutdown should translate successfully
    let cmd = AudioCommand::Shutdown;
    assert!(
        cmd.into_audio_message(&app).is_ok(),
        "Shutdown should translate"
    );

    // App commands that always pass validation
    let app_commands: Vec<AppCommand> = vec![
        AppCommand::System(SystemCommand::Reset),
        AppCommand::Live(LiveCommand::LinkOctaves),
        AppCommand::Live(LiveCommand::UnlinkOctaves),
    ];

    for cmd in app_commands {
        assert!(
            cmd.validate(&app).is_ok(),
            "AppCommand {:?} should pass validation",
            cmd
        );
    }
}

// ============================================================================
// Command Validation Tests - Invalid Commands
// ============================================================================

#[test]
fn test_notestart_invalid_row() {
    let app = App::new();

    let cmd = AudioCommand::NoteStart {
        note: 0,
        row: 2,
        velocity: 0.5,
    };
    let result = cmd.into_audio_message(&app);

    assert!(result.is_err(), "NoteStart with row=2 should fail");
    match result.unwrap_err() {
        CommandError::RowOutOfBounds(row) => {
            assert_eq!(row, 2, "Error should report row 2");
        }
        _ => panic!("Expected RowOutOfBounds error"),
    }

    let cmd = AudioCommand::NoteStart {
        note: 0,
        row: 255,
        velocity: 0.5,
    };
    let result = cmd.into_audio_message(&app);
    assert!(result.is_err(), "NoteStart with row=255 should fail");
}

#[test]
fn test_notestart_invalid_velocity_negative() {
    let app = App::new();

    let cmd = AudioCommand::NoteStart {
        note: 0,
        row: 0,
        velocity: -0.1,
    };
    let result = cmd.into_audio_message(&app);

    assert!(
        result.is_err(),
        "NoteStart with negative velocity should fail"
    );
    match result.unwrap_err() {
        CommandError::InvalidVolume(vel) => {
            assert_eq!(vel, -0.1, "Error should report velocity -0.1");
        }
        _ => panic!("Expected InvalidVolume error"),
    }
}

#[test]
fn test_notestart_invalid_velocity_excessive() {
    let app = App::new();

    let cmd = AudioCommand::NoteStart {
        note: 0,
        row: 0,
        velocity: 1.5,
    };
    let result = cmd.into_audio_message(&app);

    assert!(result.is_err(), "NoteStart with velocity > 1.0 should fail");
    match result.unwrap_err() {
        CommandError::InvalidVolume(vel) => {
            assert_eq!(vel, 1.5, "Error should report velocity 1.5");
        }
        _ => panic!("Expected InvalidVolume error"),
    }
}

#[test]
fn test_notestop_invalid_row() {
    let app = App::new();

    let cmd = AudioCommand::NoteStop { note: 0, row: 2 };
    let result = cmd.into_audio_message(&app);

    assert!(result.is_err(), "NoteStop with row=2 should fail");
    match result.unwrap_err() {
        CommandError::RowOutOfBounds(row) => {
            assert_eq!(row, 2, "Error should report row 2");
        }
        _ => panic!("Expected RowOutOfBounds error"),
    }
}

#[test]
fn test_setoctave_invalid_octave() {
    let app = App::new();

    let cmd = AppCommand::Live(LiveCommand::SetOctave { octave: 9, row: 0 });
    let result = cmd.validate(&app);

    assert!(result.is_err(), "SetOctave with octave=9 should fail");
    match result.unwrap_err() {
        CommandError::InvalidOctave(octave) => {
            assert_eq!(octave, 9, "Error should report octave 9");
        }
        _ => panic!("Expected InvalidOctave error"),
    }

    let cmd = AppCommand::Live(LiveCommand::SetOctave {
        octave: 255,
        row: 0,
    });
    let result = cmd.validate(&app);
    assert!(result.is_err(), "SetOctave with octave=255 should fail");
}

#[test]
fn test_setoctave_invalid_row() {
    let app = App::new();

    let cmd = AppCommand::Live(LiveCommand::SetOctave { octave: 4, row: 2 });
    let result = cmd.validate(&app);

    assert!(
        result.is_err(),
        "SetOctave with row=2 should fail even with valid octave"
    );
    match result.unwrap_err() {
        CommandError::RowOutOfBounds(row) => {
            assert_eq!(row, 2, "Error should report row 2");
        }
        _ => panic!("Expected RowOutOfBounds error"),
    }
}

#[test]
fn test_octaveup_invalid_row() {
    let app = App::new();

    let cmd = AppCommand::Live(LiveCommand::OctaveUp(2));
    let result = cmd.validate(&app);

    assert!(result.is_err(), "OctaveUp with row=2 should fail");
    match result.unwrap_err() {
        CommandError::RowOutOfBounds(row) => {
            assert_eq!(row, 2, "Error should report row 2");
        }
        _ => panic!("Expected RowOutOfBounds error"),
    }
}

#[test]
fn test_octavedown_invalid_row() {
    let app = App::new();

    let cmd = AppCommand::Live(LiveCommand::OctaveDown(3));
    let result = cmd.validate(&app);

    assert!(result.is_err(), "OctaveDown with row=3 should fail");
    match result.unwrap_err() {
        CommandError::RowOutOfBounds(row) => {
            assert_eq!(row, 3, "Error should report row 3");
        }
        _ => panic!("Expected RowOutOfBounds error"),
    }
}

// ============================================================================
// AudioCommand to AudioMessage Translation Tests
// ============================================================================
#[test]
fn test_translate_notestart() {
    let app = App::new();

    let cmd = AudioCommand::NoteStart {
        note: 0,
        row: 0,
        velocity: 0.7,
    };
    let audio_msg = cmd.into_audio_message(&app).unwrap();

    match audio_msg {
        AudioMessage::Instrument(InstrumentAudioMessage::NoteStart {
            instrument_idx,
            note,
            velocity,
        }) => {
            assert_eq!(instrument_idx, 0, "Instrument index should be 0");
            assert_eq!(
                note.octave(),
                3,
                "Note octave should be 3 (row 0 default in App::new)"
            );
            assert_eq!(velocity, 0.7, "Velocity should be 0.7");
        }
        _ => panic!("Expected AudioMessage::Instrument(NoteStart)"),
    }
}

#[test]
fn test_translate_notestop() {
    let mut app = App::new();
    app.rows[1].octave = 5;
    app.rows[1].instrument = 0;

    let cmd = AudioCommand::NoteStop { note: 7, row: 1 };
    let audio_msg = cmd.into_audio_message(&app).unwrap();

    match audio_msg {
        AudioMessage::Instrument(InstrumentAudioMessage::NoteStop {
            instrument_idx,
            note,
        }) => {
            assert_eq!(instrument_idx, 0, "Instrument index should be 0");
            assert_eq!(note.octave(), 5, "Note octave should be 5");
        }
        _ => panic!("Expected AudioMessage::Instrument(NoteStop)"),
    }
}

#[test]
fn test_translate_shutdown() {
    let app = App::new();

    let cmd = AudioCommand::Shutdown;
    let audio_msg = cmd.into_audio_message(&app).unwrap();

    match audio_msg {
        AudioMessage::Shutdown => {}
        _ => panic!("Expected AudioMessage::Shutdown"),
    }
}

// ============================================================================
// Error Display Tests
// ============================================================================

#[test]
fn test_error_display_row_out_of_bounds() {
    let error = CommandError::RowOutOfBounds(2);
    let error_msg = format!("{}", error);

    assert!(error_msg.contains("Row index out of bounds"));
    assert!(error_msg.contains("2"));
}

#[test]
fn test_error_display_invalid_octave() {
    let error = CommandError::InvalidOctave(9);
    let error_msg = format!("{}", error);

    assert!(error_msg.contains("Invalid octave"));
    assert!(error_msg.contains("9"));
    assert!(error_msg.contains("must be 0-8"));
}

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
    use rustic::Note;
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
