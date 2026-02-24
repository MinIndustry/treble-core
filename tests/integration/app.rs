//! Application State Management Tests
//!
//! This test suite validates application state management:
//! - App initialization with default and custom configs
//! - Configuration loading from files

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
    assert_eq!(app.config.audio.audio_ring_buffer_size, 4096);

    assert_eq!(app.config.logging.level, "info");
    assert!(!app.config.logging.log_to_file);
    assert!(app.config.logging.log_to_stdout);
}

#[test]
fn test_app_new_starts_without_instruments() {
    let app = App::new();
    assert!(
        app.instruments.is_empty(),
        "New App should start with no instruments; add them explicitly"
    );
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
