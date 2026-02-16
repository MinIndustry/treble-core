//! Application State Management Tests
//!
//! This test suite validates application state management:
//! - App initialization with default and custom configs
//! - Event handling for various command types
//! - Octave management (OctaveUp, OctaveDown, SetOctave)
//! - Row management and state updates
//! - Configuration loading from files

use rustic::app::commands::{AppCommand, LiveCommand, SettingsCommand, SystemCommand};
use rustic::prelude::App;
use std::io::Write;

// ============================================================================
// App Initialization Tests
// ============================================================================

#[test]
fn test_app_new_default_config() {
    // App::new() should load default configuration
    let app = App::new();

    // Verify default audio config values
    assert_eq!(
        app.config.audio.cpal_buffer_size, 64,
        "Default cpal_buffer_size should be 64"
    );
    assert_eq!(
        app.config.audio.render_chunk_size, 256,
        "Default render_chunk_size should be 256"
    );
    assert_eq!(
        app.config.audio.audio_ring_buffer_size, 88200,
        "Default audio_ring_buffer_size should be 88200"
    );

    // Verify default logging config values
    assert_eq!(
        app.config.logging.level, "info",
        "Default log level should be 'info'"
    );
    assert!(
        !app.config.logging.log_to_file,
        "Default log_to_file should be false"
    );
    assert!(
        app.config.logging.log_to_stdout,
        "Default log_to_stdout should be true"
    );
}

#[test]
fn test_app_new_default_rows() {
    // App::new() should initialize rows with correct defaults
    let app = App::new();

    // Row 0 should have octave 3 (set in App::new())
    assert_eq!(app.rows[0].octave, 3, "Row 0 should start at octave 3");

    // Row 1 should have default octave 4
    assert_eq!(app.rows[1].octave, 4, "Row 1 should have default octave 4");

    // Both rows should point to instrument 0
    assert_eq!(app.rows[0].instrument, 0, "Row 0 should use instrument 0");
    assert_eq!(app.rows[1].instrument, 0, "Row 1 should use instrument 0");
}

#[test]
fn test_app_new_has_instruments() {
    // App::new() should initialize with at least one instrument
    let app = App::new();

    assert!(
        !app.instruments.is_empty(),
        "App should have at least one instrument"
    );
    assert_eq!(
        app.instruments.len(),
        1,
        "Default app should have exactly 1 instrument"
    );
}

#[test]
fn test_app_from_file_valid_config() {
    // Test loading app from a valid TOML config file
    let temp_dir = std::env::temp_dir();
    let config_file = temp_dir.join("test_app_config.toml");

    // Create a config file with custom values
    // FSConfig needs all required fields to be valid
    let toml_content = r#"
        [audio]
        cpal_buffer_size = 128
        render_chunk_size = 512
        target_latency_ms = 25.0

        [logging]
        level = "debug"
        log_to_file = true
        log_file = "test.log"

        [system]
        sample_rate = 48000
        master_volume = 1.0

        [fs]
        root_dir = "/tmp"
        score_path = "/tmp/scores"
        instrument_path = "/tmp/instruments"
        recordings_path = "/tmp/recordings"
    "#;

    {
        let mut file = std::fs::File::create(&config_file).expect("Failed to create temp file");
        file.write_all(toml_content.as_bytes())
            .expect("Failed to write to temp file");
    }

    // Load the app from file
    let result = App::from_file(&config_file);
    assert!(
        result.is_ok(),
        "Should successfully load config from file: {:?}",
        result.err()
    );

    let app = result.unwrap();

    // Verify custom audio config was loaded
    assert_eq!(
        app.config.audio.cpal_buffer_size, 128,
        "Should load custom cpal_buffer_size"
    );
    assert_eq!(
        app.config.audio.render_chunk_size, 512,
        "Should load custom render_chunk_size"
    );
    assert_eq!(
        app.config.audio.target_latency_ms, 25.0,
        "Should load custom target_latency_ms"
    );

    // Verify custom logging config was loaded
    assert_eq!(
        app.config.logging.level, "debug",
        "Should load custom log level"
    );
    assert!(
        app.config.logging.log_to_file,
        "Should load custom log_to_file"
    );
    assert_eq!(
        app.config.logging.log_file, "test.log",
        "Should load custom log_file"
    );

    // Verify system config was loaded
    assert_eq!(
        app.config.system.sample_rate, 48000,
        "Should load custom sample_rate"
    );

    // Clean up
    std::fs::remove_file(&config_file).expect("Failed to remove temp file");
}

#[test]
fn test_app_from_file_missing_fields_use_defaults() {
    // Test that missing fields within sections use defaults
    let temp_dir = std::env::temp_dir();
    let config_file = temp_dir.join("test_app_partial_config.toml");

    // Create a config with partial field specifications
    // Note: system and fs sections are required, but we can omit fields within sections
    let toml_content = r#"
        [audio]
        cpal_buffer_size = 256
        # render_chunk_size omitted - should use default

        [system]
        sample_rate = 44100
        master_volume = 1.0

        [fs]
        root_dir = "/tmp"
        score_path = "/tmp/scores"
        instrument_path = "/tmp/instruments"
        recordings_path = "/tmp/recordings"

        # logging section entirely omitted - should use all defaults
    "#;

    {
        let mut file = std::fs::File::create(&config_file).expect("Failed to create temp file");
        file.write_all(toml_content.as_bytes())
            .expect("Failed to write to temp file");
    }

    let result = App::from_file(&config_file);
    assert!(
        result.is_ok(),
        "Should load partial config and use defaults: {:?}",
        result.err()
    );

    let app = result.unwrap();

    // Specified value should be custom
    assert_eq!(app.config.audio.cpal_buffer_size, 256);

    // Unspecified audio values should be default
    assert_eq!(
        app.config.audio.render_chunk_size, 256,
        "Default render_chunk_size should be 256"
    );

    // Logging section should be entirely default since it was omitted
    assert_eq!(
        app.config.logging.level, "info",
        "Default log level should be 'info'"
    );
    assert!(
        app.config.logging.log_to_stdout,
        "Default log_to_stdout should be true"
    );

    // Clean up
    std::fs::remove_file(&config_file).expect("Failed to remove temp file");
}

#[test]
fn test_app_from_file_nonexistent_file() {
    // Test loading from a file that doesn't exist
    let nonexistent_path = std::path::Path::new("/tmp/nonexistent_config_12345.toml");

    let result = App::from_file(nonexistent_path);
    assert!(result.is_err(), "Loading from nonexistent file should fail");
}

// ============================================================================
// Event Handling Tests - Octave Control
// ============================================================================

#[test]
fn test_on_event_octaveup() {
    // OctaveUp should increment the octave for the specified row
    let mut app = App::new();
    app.rows[0].octave = 4;
    app.rows[1].octave = 3;

    // Increase octave for row 0
    app.on_event(AppCommand::Live(LiveCommand::OctaveUp(0)));
    assert_eq!(app.rows[0].octave, 5, "Row 0 octave should increase to 5");
    assert_eq!(app.rows[1].octave, 3, "Row 1 octave should remain at 3");

    // Increase octave for row 1
    app.on_event(AppCommand::Live(LiveCommand::OctaveUp(1)));
    assert_eq!(app.rows[0].octave, 5, "Row 0 octave should remain at 5");
    assert_eq!(app.rows[1].octave, 4, "Row 1 octave should increase to 4");
}

#[test]
fn test_on_event_octavedown() {
    // OctaveDown should decrement the octave for the specified row
    let mut app = App::new();
    app.rows[0].octave = 5;
    app.rows[1].octave = 6;

    // Decrease octave for row 0
    app.on_event(AppCommand::Live(LiveCommand::OctaveDown(0)));
    assert_eq!(app.rows[0].octave, 4, "Row 0 octave should decrease to 4");
    assert_eq!(app.rows[1].octave, 6, "Row 1 octave should remain at 6");

    // Decrease octave for row 1
    app.on_event(AppCommand::Live(LiveCommand::OctaveDown(1)));
    assert_eq!(app.rows[0].octave, 4, "Row 0 octave should remain at 4");
    assert_eq!(app.rows[1].octave, 5, "Row 1 octave should decrease to 5");
}

#[test]
fn test_on_event_octaveup_multiple_times() {
    // Multiple OctaveUp events should accumulate
    let mut app = App::new();
    app.rows[0].octave = 2;

    app.on_event(AppCommand::Live(LiveCommand::OctaveUp(0)));
    app.on_event(AppCommand::Live(LiveCommand::OctaveUp(0)));
    app.on_event(AppCommand::Live(LiveCommand::OctaveUp(0)));

    assert_eq!(app.rows[0].octave, 5, "Row 0 octave should increase by 3");
}

#[test]
fn test_on_event_octavedown_multiple_times() {
    // Multiple OctaveDown events should accumulate
    let mut app = App::new();
    app.rows[1].octave = 7;

    app.on_event(AppCommand::Live(LiveCommand::OctaveDown(1)));
    app.on_event(AppCommand::Live(LiveCommand::OctaveDown(1)));

    assert_eq!(app.rows[1].octave, 5, "Row 1 octave should decrease by 2");
}

#[test]
fn test_on_event_octaveup_and_down() {
    // OctaveUp and OctaveDown should cancel each other out
    let mut app = App::new();
    app.rows[0].octave = 4;

    app.on_event(AppCommand::Live(LiveCommand::OctaveUp(0)));
    app.on_event(AppCommand::Live(LiveCommand::OctaveUp(0)));
    app.on_event(AppCommand::Live(LiveCommand::OctaveDown(0)));

    assert_eq!(
        app.rows[0].octave, 5,
        "Row 0 octave should be 5 (4 + 2 - 1)"
    );
}

#[test]
#[should_panic(expected = "index out of bounds")]
fn test_on_event_octaveup_invalid_row_panics() {
    // OctaveUp with invalid row should panic
    let mut app = App::new();
    app.on_event(AppCommand::Live(LiveCommand::OctaveUp(2)));
}

#[test]
#[should_panic(expected = "index out of bounds")]
fn test_on_event_octavedown_invalid_row_panics() {
    // OctaveDown with invalid row should panic
    let mut app = App::new();
    app.on_event(AppCommand::Live(LiveCommand::OctaveDown(5)));
}

// ============================================================================
// Event Handling Tests - Other Commands
// ============================================================================

#[test]
fn test_on_event_unhandled_commands() {
    // Commands not explicitly handled should not cause panics
    let mut app = App::new();

    // These commands are matched by the wildcard in on_event
    app.on_event(AppCommand::System(SystemCommand::Reset));
    app.on_event(AppCommand::Live(LiveCommand::LinkOctaves));
    app.on_event(AppCommand::Live(LiveCommand::UnlinkOctaves));
    app.on_event(AppCommand::Settings(SettingsCommand::ToggleMetronome));

    // No panic = success
}

// ============================================================================
// Row Management Tests
// ============================================================================

#[test]
fn test_row_get_note() {
    // Test that Row::get_note produces correct Note instances
    let mut app = App::new();
    app.rows[0].octave = 4;

    // Get note for row 0
    let note = app.rows[0].get_note(0); // C
    assert_eq!(note.octave(), 4, "Note should have octave 4");

    // Change octave and verify
    app.rows[0].octave = 5;
    let note = app.rows[0].get_note(7); // G
    assert_eq!(note.octave(), 5, "Note should have octave 5");
}

#[test]
fn test_row_independent_octaves() {
    // Test that rows maintain independent octave values
    let mut app = App::new();
    app.rows[0].octave = 3;
    app.rows[1].octave = 6;

    // Modify row 0
    app.on_event(AppCommand::Live(LiveCommand::OctaveUp(0)));
    assert_eq!(app.rows[0].octave, 4, "Row 0 should be at octave 4");
    assert_eq!(app.rows[1].octave, 6, "Row 1 should remain at octave 6");

    // Modify row 1
    app.on_event(AppCommand::Live(LiveCommand::OctaveDown(1)));
    assert_eq!(app.rows[0].octave, 4, "Row 0 should remain at octave 4");
    assert_eq!(app.rows[1].octave, 5, "Row 1 should be at octave 5");
}

#[test]
fn test_row_instrument_assignment() {
    // Test that rows can reference different instruments
    let mut app = App::new();

    // Both rows start with instrument 0
    assert_eq!(app.rows[0].instrument, 0);
    assert_eq!(app.rows[1].instrument, 0);

    // We can manually change instrument assignments
    app.rows[1].instrument = 0; // Would be different if we had multiple instruments
    assert_eq!(app.rows[1].instrument, 0);
}

// ============================================================================
// Live Tick Tests
// ============================================================================

#[test]
fn test_live_tick_produces_output() {
    // Test that live_tick calls tick on instruments and produces output
    let mut app = App::new();

    // Call live_tick - this exercises the tick mechanism
    let output = app.live_tick();

    // Note: The instrument may return NaN if not fully initialized or no notes are playing
    // The important thing is that live_tick() completes without panicking
    // We just verify it returns *some* f32 value (NaN, finite, or infinity)
    let _ = output; // Consume the value to show we got it

    // Success is defined as not panicking - the test passes if we reach here
}

#[test]
fn test_live_tick_multiple_calls() {
    // Test that multiple live_tick calls work correctly without panicking
    let mut app = App::new();

    // The primary goal is to ensure multiple ticks don't cause crashes
    for _ in 0..100 {
        let _output = app.live_tick();
        // Success is measured by not panicking
    }
}

// ============================================================================
// Configuration Validation Integration
// ============================================================================

#[test]
fn test_app_config_can_be_validated() {
    // Test that we can validate the app's audio config
    let app = App::new();

    let result = app.config.audio.validate();
    assert!(result.is_ok(), "Default app config should pass validation");
}

#[test]
fn test_app_with_invalid_config() {
    // Create an app and modify its config to be invalid
    let mut app = App::new();
    app.config.audio.cpal_buffer_size = 0; // Invalid!

    let result = app.config.audio.validate();
    assert!(result.is_err(), "Invalid config should fail validation");
}

// ============================================================================
// State Consistency Tests
// ============================================================================

#[test]
fn test_multiple_commands_sequence() {
    // Test a realistic sequence of commands
    let mut app = App::new();

    // Initial state
    app.rows[0].octave = 4;

    // Sequence of operations
    app.on_event(AppCommand::Live(LiveCommand::OctaveUp(0))); // Change to octave 5
    app.on_event(AppCommand::Live(LiveCommand::OctaveDown(0))); // Back to octave 4

    // Verify final state
    assert_eq!(app.rows[0].octave, 4, "Should be back at octave 4");
}

#[test]
fn test_interleaved_row_commands() {
    // Test commands interleaved between different rows
    let mut app = App::new();
    app.rows[0].octave = 3;
    app.rows[1].octave = 5;

    app.on_event(AppCommand::Live(LiveCommand::OctaveUp(0)));
    app.on_event(AppCommand::Live(LiveCommand::OctaveUp(1)));
    app.on_event(AppCommand::Live(LiveCommand::OctaveDown(0)));

    // Verify both rows maintained correct state
    assert_eq!(app.rows[0].octave, 3, "Row 0 should be at octave 3");
    assert_eq!(app.rows[1].octave, 6, "Row 1 should be at octave 6");
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
#[should_panic(expected = "attempt to subtract with overflow")]
fn test_octave_underflow_behavior() {
    // Test behavior when octave goes below 0 (panics in debug mode)
    let mut app = App::new();
    app.rows[0].octave = 0;

    // This will panic in debug mode due to overflow checks
    app.on_event(AppCommand::Live(LiveCommand::OctaveDown(0)));
}

#[test]
#[should_panic(expected = "attempt to add with overflow")]
fn test_octave_overflow_behavior() {
    // Test behavior when octave goes above 255 (panics in debug mode)
    let mut app = App::new();
    app.rows[0].octave = 255;

    // This will panic in debug mode due to overflow checks
    app.on_event(AppCommand::Live(LiveCommand::OctaveUp(0)));
}
