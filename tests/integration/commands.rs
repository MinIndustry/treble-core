//! Command Validation and Translation Tests
//!
//! This test suite validates the command validation and translation system:
//! - Command validation for valid inputs
//! - Command validation failure for invalid inputs
//! - Translation from Commands to AudioMessage
//! - Error types and messages

use rustic::app::commands::{
    Command, LiveCommand, LoopCommand, MixCommand, SettingsCommand, SystemCommand,
};
use rustic::audio::messages::InstrumentAudioMessage;
use rustic::audio::{AudioMessage, CommandError};
use rustic::prelude::App;

// ============================================================================
// Command Validation Tests - Valid Commands
// ============================================================================

#[test]
fn test_notestart_valid_parameters() {
    // NoteStart with valid parameters should pass validation
    let app = App::new();

    // Valid: row 0, note 0 (C), velocity 0.5
    let cmd = Command::Live(LiveCommand::NoteStart {
        note: 0,
        row: 0,
        velocity: 0.5,
    });
    assert!(
        cmd.validate(&app).is_ok(),
        "NoteStart with valid parameters should pass"
    );

    // Valid: row 1, note 11 (B), velocity 1.0
    let cmd = Command::Live(LiveCommand::NoteStart {
        note: 11,
        row: 1,
        velocity: 1.0,
    });
    assert!(
        cmd.validate(&app).is_ok(),
        "NoteStart with max velocity should pass"
    );

    // Valid: row 0, note 5 (F), velocity 0.0
    let cmd = Command::Live(LiveCommand::NoteStart {
        note: 5,
        row: 0,
        velocity: 0.0,
    });
    assert!(
        cmd.validate(&app).is_ok(),
        "NoteStart with zero velocity should pass"
    );
}

#[test]
fn test_notestop_valid_parameters() {
    // NoteStop with valid parameters should pass validation
    let app = App::new();

    // Valid: row 0, note 0
    let cmd = Command::Live(LiveCommand::NoteStop { note: 0, row: 0 });
    assert!(
        cmd.validate(&app).is_ok(),
        "NoteStop with valid row should pass"
    );

    // Valid: row 1, note 11
    let cmd = Command::Live(LiveCommand::NoteStop { note: 11, row: 1 });
    assert!(
        cmd.validate(&app).is_ok(),
        "NoteStop with row 1 should pass"
    );
}

#[test]
fn test_setoctave_valid_parameters() {
    // SetOctave with valid parameters should pass validation
    let app = App::new();

    // Valid: octave 0-8, row 0-1
    for octave in 0..=8 {
        for row in 0..=1 {
            let cmd = Command::Live(LiveCommand::SetOctave { octave, row });
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
    // OctaveUp with valid row should pass validation
    let app = App::new();

    let cmd = Command::Live(LiveCommand::OctaveUp(0));
    assert!(cmd.validate(&app).is_ok(), "OctaveUp(0) should pass");

    let cmd = Command::Live(LiveCommand::OctaveUp(1));
    assert!(cmd.validate(&app).is_ok(), "OctaveUp(1) should pass");
}

#[test]
fn test_octavedown_valid_parameters() {
    // OctaveDown with valid row should pass validation
    let app = App::new();

    let cmd = Command::Live(LiveCommand::OctaveDown(0));
    assert!(cmd.validate(&app).is_ok(), "OctaveDown(0) should pass");

    let cmd = Command::Live(LiveCommand::OctaveDown(1));
    assert!(cmd.validate(&app).is_ok(), "OctaveDown(1) should pass");
}

#[test]
fn test_commands_without_validation() {
    // Commands that don't require validation should always pass
    let app = App::new();

    let commands_to_test: Vec<Command> = vec![
        Command::System(SystemCommand::Quit),
        Command::System(SystemCommand::Reset),
        Command::Live(LiveCommand::LinkOctaves),
        Command::Live(LiveCommand::UnlinkOctaves),
        Command::Settings(SettingsCommand::ToggleMetronome),
        Command::Settings(SettingsCommand::ToggleHelp),
        Command::Mix(MixCommand::MuteAll),
    ];

    for cmd in commands_to_test {
        assert!(
            cmd.validate(&app).is_ok(),
            "Command {:?} should pass validation",
            cmd
        );
    }
}

// ============================================================================
// Command Validation Tests - Invalid Commands
// ============================================================================

#[test]
fn test_notestart_invalid_row() {
    // NoteStart with row >= 2 should fail with RowOutOfBounds error
    let app = App::new();

    let cmd = Command::Live(LiveCommand::NoteStart {
        note: 0,
        row: 2,
        velocity: 0.5,
    });
    let result = cmd.validate(&app);

    assert!(result.is_err(), "NoteStart with row=2 should fail");
    match result.unwrap_err() {
        CommandError::RowOutOfBounds(row) => {
            assert_eq!(row, 2, "Error should report row 2");
        }
        _ => panic!("Expected RowOutOfBounds error"),
    }

    // Test with row = 255
    let cmd = Command::Live(LiveCommand::NoteStart {
        note: 0,
        row: 255,
        velocity: 0.5,
    });
    let result = cmd.validate(&app);
    assert!(result.is_err(), "NoteStart with row=255 should fail");
}

#[test]
fn test_notestart_invalid_velocity_negative() {
    // NoteStart with velocity < 0.0 should fail with InvalidVolume error
    let app = App::new();

    let cmd = Command::Live(LiveCommand::NoteStart {
        note: 0,
        row: 0,
        velocity: -0.1,
    });
    let result = cmd.validate(&app);

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
    // NoteStart with velocity > 1.0 should fail with InvalidVolume error
    let app = App::new();

    let cmd = Command::Live(LiveCommand::NoteStart {
        note: 0,
        row: 0,
        velocity: 1.5,
    });
    let result = cmd.validate(&app);

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
    // NoteStop with row >= 2 should fail with RowOutOfBounds error
    let app = App::new();

    let cmd = Command::Live(LiveCommand::NoteStop { note: 0, row: 2 });
    let result = cmd.validate(&app);

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
    // SetOctave with octave > 8 should fail with InvalidOctave error
    let app = App::new();

    let cmd = Command::Live(LiveCommand::SetOctave { octave: 9, row: 0 });
    let result = cmd.validate(&app);

    assert!(result.is_err(), "SetOctave with octave=9 should fail");
    match result.unwrap_err() {
        CommandError::InvalidOctave(octave) => {
            assert_eq!(octave, 9, "Error should report octave 9");
        }
        _ => panic!("Expected InvalidOctave error"),
    }

    // Test with octave = 255
    let cmd = Command::Live(LiveCommand::SetOctave {
        octave: 255,
        row: 0,
    });
    let result = cmd.validate(&app);
    assert!(result.is_err(), "SetOctave with octave=255 should fail");
}

#[test]
fn test_setoctave_invalid_row() {
    // SetOctave with row >= 2 should fail with RowOutOfBounds error
    let app = App::new();

    let cmd = Command::Live(LiveCommand::SetOctave { octave: 4, row: 2 });
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
    // OctaveUp with row >= 2 should fail with RowOutOfBounds error
    let app = App::new();

    let cmd = Command::Live(LiveCommand::OctaveUp(2));
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
    // OctaveDown with row >= 2 should fail with RowOutOfBounds error
    let app = App::new();

    let cmd = Command::Live(LiveCommand::OctaveDown(3));
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
// Command to AudioMessage Translation Tests
// ============================================================================
#[test]
fn test_translate_notestart() {
    // NoteStart should translate to AudioMessage::NoteStart
    let mut app = App::new();
    app.rows[0].octave = 4;
    app.rows[0].instrument = 0;

    let cmd = Command::Live(LiveCommand::NoteStart {
        note: 0,
        row: 0,
        velocity: 0.7,
    });
    let audio_msg = cmd.translate_to_audio_message(&mut app);

    assert!(audio_msg.is_some(), "NoteStart should produce AudioMessage");

    match audio_msg.unwrap() {
        AudioMessage::Instrument(InstrumentAudioMessage::NoteStart {
            instrument_idx,
            note,
            velocity,
        }) => {
            assert_eq!(instrument_idx, 0, "Instrument index should be 0");
            assert_eq!(note.octave(), 4, "Note octave should be 4");
            assert_eq!(velocity, 0.7, "Velocity should be 0.7");
        }
        _ => panic!("Expected AudioMessage::Instrument(NoteStart)"),
    }
}

#[test]
fn test_translate_notestop() {
    // NoteStop should translate to AudioMessage::NoteStop
    let mut app = App::new();
    app.rows[1].octave = 5;
    app.rows[1].instrument = 0;

    let cmd = Command::Live(LiveCommand::NoteStop { note: 7, row: 1 });
    let audio_msg = cmd.translate_to_audio_message(&mut app);

    assert!(audio_msg.is_some(), "NoteStop should produce AudioMessage");

    match audio_msg.unwrap() {
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
fn test_translate_setoctave() {
    // SetOctave should translate to AudioMessage::SetOctave after updating app state
    let mut app = App::new();
    app.rows[0].octave = 6;

    let cmd = Command::Live(LiveCommand::SetOctave { octave: 3, row: 0 });
    // Note: The command validation happens before translation,
    // but translation reads from the app state which should be updated

    let audio_msg = cmd.translate_to_audio_message(&mut app);

    assert!(audio_msg.is_some(), "SetOctave should produce AudioMessage");

    match audio_msg.unwrap() {
        AudioMessage::Instrument(InstrumentAudioMessage::SetOctave { row, octave }) => {
            assert_eq!(row, 0, "Row should be 0");
            // The octave in the message reflects the current app state
            // which is 6 since we haven't actually updated it yet
            assert_eq!(octave, 6, "Octave should match app state");
        }
        _ => panic!("Expected AudioMessage::Instrument(SetOctave)"),
    }
}

#[test]
fn test_translate_octaveup() {
    // OctaveUp should translate to AudioMessage::SetOctave
    let mut app = App::new();
    app.rows[1].octave = 4;

    let cmd = Command::Live(LiveCommand::OctaveUp(1));
    let audio_msg = cmd.translate_to_audio_message(&mut app);

    assert!(audio_msg.is_some(), "OctaveUp should produce AudioMessage");

    match audio_msg.unwrap() {
        AudioMessage::Instrument(InstrumentAudioMessage::SetOctave { row, octave }) => {
            assert_eq!(row, 1, "Row should be 1");
            assert_eq!(octave, 4, "Octave should match current app state");
        }
        _ => panic!("Expected AudioMessage::Instrument(SetOctave)"),
    }
}

#[test]
fn test_translate_octavedown() {
    // OctaveDown should translate to AudioMessage::SetOctave
    let mut app = App::new();
    app.rows[0].octave = 5;

    let cmd = Command::Live(LiveCommand::OctaveDown(0));
    let audio_msg = cmd.translate_to_audio_message(&mut app);

    assert!(
        audio_msg.is_some(),
        "OctaveDown should produce AudioMessage"
    );

    match audio_msg.unwrap() {
        AudioMessage::Instrument(InstrumentAudioMessage::SetOctave { row, octave }) => {
            assert_eq!(row, 0, "Row should be 0");
            assert_eq!(octave, 5, "Octave should match current app state");
        }
        _ => panic!("Expected AudioMessage::Instrument(SetOctave)"),
    }
}

#[test]
fn test_translate_quit() {
    // Quit should translate to AudioMessage::Shutdown
    let mut app = App::new();

    let cmd = Command::System(SystemCommand::Quit);
    let audio_msg = cmd.translate_to_audio_message(&mut app);

    assert!(audio_msg.is_some(), "Quit should produce AudioMessage");

    match audio_msg.unwrap() {
        AudioMessage::Shutdown => {
            // Expected
        }
        _ => panic!("Expected AudioMessage::Shutdown"),
    }
}

#[test]
fn test_translate_commands_without_audio_message() {
    // Commands that don't produce audio messages should return None
    let mut app = App::new();

    let commands_without_audio_msg: Vec<Command> = vec![
        Command::System(SystemCommand::Reset),
        Command::Live(LiveCommand::LinkOctaves),
        Command::Live(LiveCommand::UnlinkOctaves),
        Command::Loop(LoopCommand::StartRecording),
        Command::Loop(LoopCommand::StopRecording),
        Command::Loop(LoopCommand::PlayLoop),
        Command::Loop(LoopCommand::StopLoop),
        Command::Settings(SettingsCommand::ToggleMetronome),
        Command::Settings(SettingsCommand::ToggleHelp),
    ];

    for cmd in commands_without_audio_msg {
        let audio_msg = cmd.translate_to_audio_message(&mut app);
        assert!(
            audio_msg.is_none(),
            "Command {:?} should not produce AudioMessage",
            cmd
        );
    }
}

// ============================================================================
// Error Display Tests
// ============================================================================

#[test]
fn test_error_display_row_out_of_bounds() {
    // Test the display format of RowOutOfBounds error
    let error = CommandError::RowOutOfBounds(2);
    let error_msg = format!("{}", error);

    assert!(
        error_msg.contains("Row index out of bounds"),
        "Error message should describe the error type"
    );
    assert!(
        error_msg.contains("2"),
        "Error message should include the invalid row"
    );
}

#[test]
fn test_error_display_invalid_octave() {
    // Test the display format of InvalidOctave error
    let error = CommandError::InvalidOctave(9);
    let error_msg = format!("{}", error);

    assert!(
        error_msg.contains("Invalid octave"),
        "Error message should describe the error type"
    );
    assert!(
        error_msg.contains("9"),
        "Error message should include the invalid octave"
    );
    assert!(
        error_msg.contains("must be 0-8"),
        "Error message should describe valid range"
    );
}

#[test]
fn test_error_display_invalid_volume() {
    // Test the display format of InvalidVolume error
    let error = CommandError::InvalidVolume(1.5);
    let error_msg = format!("{}", error);

    assert!(
        error_msg.contains("Invalid volume"),
        "Error message should describe the error type"
    );
    assert!(
        error_msg.contains("1.5"),
        "Error message should include the invalid volume"
    );
    assert!(
        error_msg.contains("must be 0.0-1.0"),
        "Error message should describe valid range"
    );
}

// ============================================================================
// AudioMessage Cloning Tests
// ============================================================================

#[test]
fn test_audiomessage_clone() {
    // Verify that AudioMessage variants can be cloned
    use rustic::Note;
    use rustic::audio::messages::InstrumentAudioMessage;
    use rustic::core::utils::NOTES;

    let original = AudioMessage::Instrument(InstrumentAudioMessage::NoteStart {
        instrument_idx: 0,
        note: Note(NOTES::A, 4),
        velocity: 0.8,
    });

    let cloned = original.clone();

    // Both should produce the same debug output
    assert_eq!(
        format!("{:?}", original),
        format!("{:?}", cloned),
        "Cloned AudioMessage should be identical"
    );
}
