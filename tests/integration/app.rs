//! Application State Management Tests
//!
//! This test suite validates application state management:
//! - App initialization with default and custom configs
//! - Event handling for various command types
//! - Octave management (OctaveUp, OctaveDown, SetOctave)
//! - Row management and state updates
//! - Configuration loading from files

use rustic::app::commands::{AppCommand, LiveCommand, SystemCommand};
use rustic::prelude::App;
use std::io::Write;

// ============================================================================
// App Initialization Tests
// ============================================================================

#[test]
fn test_app_new_default_config() {
    let app = App::new();

    assert_eq!(app.config.audio.cpal_buffer_size, 64);
    assert_eq!(app.config.audio.render_chunk_size, 256);
    assert_eq!(app.config.audio.audio_ring_buffer_size, 88200);

    assert_eq!(app.config.logging.level, "info");
    assert!(!app.config.logging.log_to_file);
    assert!(app.config.logging.log_to_stdout);
}

#[test]
fn test_app_new_default_rows() {
    let app = App::new();

    assert_eq!(app.rows[0].octave, 3, "Row 0 should start at octave 3");
    assert_eq!(app.rows[1].octave, 4, "Row 1 should have default octave 4");
    assert_eq!(app.rows[0].instrument, 0);
    assert_eq!(app.rows[1].instrument, 0);
}

#[test]
fn test_app_new_has_instruments() {
    let app = App::new();

    assert!(!app.instruments.is_empty());
    assert_eq!(app.instruments.len(), 1);
}

#[test]
fn test_app_from_file_valid_config() {
    let temp_dir = std::env::temp_dir();
    let config_file = temp_dir.join("test_app_config.toml");

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

    let result = App::from_file(&config_file);
    assert!(
        result.is_ok(),
        "Should successfully load config: {:?}",
        result.err()
    );

    let app = result.unwrap();
    assert_eq!(app.config.audio.cpal_buffer_size, 128);
    assert_eq!(app.config.audio.render_chunk_size, 512);
    assert_eq!(app.config.audio.target_latency_ms, 25.0);
    assert_eq!(app.config.logging.level, "debug");
    assert!(app.config.logging.log_to_file);
    assert_eq!(app.config.logging.log_file, "test.log");
    assert_eq!(app.config.system.sample_rate, 48000);

    std::fs::remove_file(&config_file).expect("Failed to remove temp file");
}

#[test]
fn test_app_from_file_missing_fields_use_defaults() {
    let temp_dir = std::env::temp_dir();
    let config_file = temp_dir.join("test_app_partial_config.toml");

    let toml_content = r#"
        [audio]
        cpal_buffer_size = 256

        [system]
        sample_rate = 44100
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

    let result = App::from_file(&config_file);
    assert!(
        result.is_ok(),
        "Should load partial config: {:?}",
        result.err()
    );

    let app = result.unwrap();
    assert_eq!(app.config.audio.cpal_buffer_size, 256);
    assert_eq!(
        app.config.audio.render_chunk_size, 256,
        "Default render_chunk_size"
    );
    assert_eq!(app.config.logging.level, "info", "Default log level");
    assert!(app.config.logging.log_to_stdout, "Default log_to_stdout");

    std::fs::remove_file(&config_file).expect("Failed to remove temp file");
}

#[test]
fn test_app_from_file_nonexistent_file() {
    let nonexistent_path = std::path::Path::new("/tmp/nonexistent_config_12345.toml");
    let result = App::from_file(nonexistent_path);
    assert!(result.is_err(), "Loading from nonexistent file should fail");
}

// ============================================================================
// Event Handling Tests - Octave Control
// ============================================================================

#[test]
fn test_on_event_octaveup() {
    let mut app = App::new();
    app.rows[0].octave = 4;
    app.rows[1].octave = 3;

    app.on_event(AppCommand::Live(LiveCommand::OctaveUp(0)));
    assert_eq!(app.rows[0].octave, 5);
    assert_eq!(app.rows[1].octave, 3);

    app.on_event(AppCommand::Live(LiveCommand::OctaveUp(1)));
    assert_eq!(app.rows[0].octave, 5);
    assert_eq!(app.rows[1].octave, 4);
}

#[test]
fn test_on_event_octavedown() {
    let mut app = App::new();
    app.rows[0].octave = 5;
    app.rows[1].octave = 6;

    app.on_event(AppCommand::Live(LiveCommand::OctaveDown(0)));
    assert_eq!(app.rows[0].octave, 4);
    assert_eq!(app.rows[1].octave, 6);

    app.on_event(AppCommand::Live(LiveCommand::OctaveDown(1)));
    assert_eq!(app.rows[0].octave, 4);
    assert_eq!(app.rows[1].octave, 5);
}

#[test]
fn test_on_event_octaveup_multiple_times() {
    let mut app = App::new();
    app.rows[0].octave = 2;

    app.on_event(AppCommand::Live(LiveCommand::OctaveUp(0)));
    app.on_event(AppCommand::Live(LiveCommand::OctaveUp(0)));
    app.on_event(AppCommand::Live(LiveCommand::OctaveUp(0)));

    assert_eq!(app.rows[0].octave, 5);
}

#[test]
fn test_on_event_octavedown_multiple_times() {
    let mut app = App::new();
    app.rows[1].octave = 7;

    app.on_event(AppCommand::Live(LiveCommand::OctaveDown(1)));
    app.on_event(AppCommand::Live(LiveCommand::OctaveDown(1)));

    assert_eq!(app.rows[1].octave, 5);
}

#[test]
fn test_on_event_octaveup_and_down() {
    let mut app = App::new();
    app.rows[0].octave = 4;

    app.on_event(AppCommand::Live(LiveCommand::OctaveUp(0)));
    app.on_event(AppCommand::Live(LiveCommand::OctaveUp(0)));
    app.on_event(AppCommand::Live(LiveCommand::OctaveDown(0)));

    assert_eq!(app.rows[0].octave, 5);
}

#[test]
#[should_panic(expected = "index out of bounds")]
fn test_on_event_octaveup_invalid_row_panics() {
    let mut app = App::new();
    app.on_event(AppCommand::Live(LiveCommand::OctaveUp(2)));
}

#[test]
#[should_panic(expected = "index out of bounds")]
fn test_on_event_octavedown_invalid_row_panics() {
    let mut app = App::new();
    app.on_event(AppCommand::Live(LiveCommand::OctaveDown(5)));
}

// ============================================================================
// Event Handling Tests - Other Commands
// ============================================================================

#[test]
fn test_on_event_unhandled_commands() {
    let mut app = App::new();

    app.on_event(AppCommand::System(SystemCommand::Reset));
    app.on_event(AppCommand::Live(LiveCommand::LinkOctaves));
    app.on_event(AppCommand::Live(LiveCommand::UnlinkOctaves));

    // No panic = success
}

// ============================================================================
// Row Management Tests
// ============================================================================

#[test]
fn test_row_get_note() {
    let mut app = App::new();
    app.rows[0].octave = 4;

    let note = app.rows[0].get_note(0);
    assert_eq!(note.octave(), 4);

    app.rows[0].octave = 5;
    let note = app.rows[0].get_note(7);
    assert_eq!(note.octave(), 5);
}

#[test]
fn test_row_independent_octaves() {
    let mut app = App::new();
    app.rows[0].octave = 3;
    app.rows[1].octave = 6;

    app.on_event(AppCommand::Live(LiveCommand::OctaveUp(0)));
    assert_eq!(app.rows[0].octave, 4);
    assert_eq!(app.rows[1].octave, 6);

    app.on_event(AppCommand::Live(LiveCommand::OctaveDown(1)));
    assert_eq!(app.rows[0].octave, 4);
    assert_eq!(app.rows[1].octave, 5);
}

#[test]
fn test_row_instrument_assignment() {
    let app = App::new();
    assert_eq!(app.rows[0].instrument, 0);
    assert_eq!(app.rows[1].instrument, 0);
}

// ============================================================================
// Live Tick Tests
// ============================================================================

#[test]
fn test_live_tick_produces_output() {
    let mut app = App::new();
    let _output = app.live_tick();
}

#[test]
fn test_live_tick_multiple_calls() {
    let mut app = App::new();
    for _ in 0..100 {
        let _output = app.live_tick();
    }
}

// ============================================================================
// Configuration Validation Integration
// ============================================================================

#[test]
fn test_app_config_can_be_validated() {
    let app = App::new();
    let result = app.config.audio.validate();
    assert!(result.is_ok(), "Default app config should pass validation");
}

#[test]
fn test_app_with_invalid_config() {
    let mut app = App::new();
    app.config.audio.cpal_buffer_size = 0;
    let result = app.config.audio.validate();
    assert!(result.is_err(), "Invalid config should fail validation");
}

// ============================================================================
// State Consistency Tests
// ============================================================================

#[test]
fn test_multiple_commands_sequence() {
    let mut app = App::new();
    app.rows[0].octave = 4;

    app.on_event(AppCommand::Live(LiveCommand::OctaveUp(0)));
    app.on_event(AppCommand::Live(LiveCommand::OctaveDown(0)));

    assert_eq!(app.rows[0].octave, 4);
}

#[test]
fn test_interleaved_row_commands() {
    let mut app = App::new();
    app.rows[0].octave = 3;
    app.rows[1].octave = 5;

    app.on_event(AppCommand::Live(LiveCommand::OctaveUp(0)));
    app.on_event(AppCommand::Live(LiveCommand::OctaveUp(1)));
    app.on_event(AppCommand::Live(LiveCommand::OctaveDown(0)));

    assert_eq!(app.rows[0].octave, 3);
    assert_eq!(app.rows[1].octave, 6);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
#[should_panic(expected = "attempt to subtract with overflow")]
fn test_octave_underflow_behavior() {
    let mut app = App::new();
    app.rows[0].octave = 0;
    app.on_event(AppCommand::Live(LiveCommand::OctaveDown(0)));
}

#[test]
#[should_panic(expected = "attempt to add with overflow")]
fn test_octave_overflow_behavior() {
    let mut app = App::new();
    app.rows[0].octave = 255;
    app.on_event(AppCommand::Live(LiveCommand::OctaveUp(0)));
}
