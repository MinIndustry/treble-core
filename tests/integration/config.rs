//! Configuration Tests
//!
//! This test suite validates the audio configuration system, including:
//! - Default values for AudioConfig and LogConfig
//! - TOML serialization/deserialization
//! - Configuration validation
//! - File loading and missing field handling

use rustic::audio::{AudioConfig, LogConfig};
use std::io::Write;

// ============================================================================
// AudioConfig Tests
// ============================================================================

#[test]
fn test_audioconfig_default_values() {
    // Validate that AudioConfig::default() returns the expected default values
    let config = AudioConfig::default();

    assert_eq!(
        config.cpal_buffer_size, 64,
        "Default cpal_buffer_size should be 64"
    );
    assert_eq!(
        config.render_chunk_size, 256,
        "Default render_chunk_size should be 256"
    );
    assert_eq!(
        config.audio_ring_buffer_size, 88200,
        "Default audio_ring_buffer_size should be 88200 (2s @ 44.1kHz)"
    );
    assert_eq!(
        config.message_ring_buffer_size, 1024,
        "Default message_ring_buffer_size should be 1024"
    );
    assert_eq!(
        config.target_latency_ms, 50.0,
        "Default target_latency_ms should be 50.0"
    );
}

#[test]
fn test_audioconfig_calculate_ring_buffer_size() {
    // Test the ring buffer size calculation based on sample rate and target latency
    let config = AudioConfig::default();

    // At 44100 Hz with 50ms target latency: (44100 * 50) / 1000 = 2205 samples
    let buffer_size = config.calculate_ring_buffer_size(44100);
    assert_eq!(
        buffer_size, 2205,
        "Ring buffer size calculation incorrect for 44.1kHz"
    );

    // At 48000 Hz with 50ms target latency: (48000 * 50) / 1000 = 2400 samples
    let buffer_size = config.calculate_ring_buffer_size(48000);
    assert_eq!(
        buffer_size, 2400,
        "Ring buffer size calculation incorrect for 48kHz"
    );
}

#[test]
fn test_audioconfig_validate_valid() {
    // Test that a valid configuration passes validation
    let config = AudioConfig::default();

    let result = config.validate();
    assert!(
        result.is_ok(),
        "Default configuration should pass validation"
    );
}

#[test]
fn test_audioconfig_validate_zero_cpal_buffer_size() {
    // Buffer size of zero should fail validation
    let config = AudioConfig {
        cpal_buffer_size: 0,
        ..Default::default()
    };

    let result = config.validate();
    assert!(result.is_err(), "Zero cpal_buffer_size should fail");
    assert!(
        result
            .unwrap_err()
            .contains("cpal_buffer_size must be between 1 and 2048")
    );
}

#[test]
fn test_audioconfig_validate_excessive_cpal_buffer_size() {
    // Buffer size over 2048 should fail validation
    let config = AudioConfig {
        cpal_buffer_size: 4096,
        ..Default::default()
    };

    let result = config.validate();
    assert!(result.is_err(), "Excessive cpal_buffer_size should fail");
    assert!(
        result
            .unwrap_err()
            .contains("cpal_buffer_size must be between 1 and 2048")
    );
}

#[test]
fn test_audioconfig_validate_render_chunk_smaller_than_cpal_buffer() {
    // Render chunk size must be >= cpal buffer size
    let config = AudioConfig {
        cpal_buffer_size: 256,
        render_chunk_size: 128,
        ..Default::default()
    };

    let result = config.validate();
    assert!(
        result.is_err(),
        "render_chunk_size < cpal_buffer_size should fail"
    );
    assert!(
        result
            .unwrap_err()
            .contains("render_chunk_size must be >= cpal_buffer_size")
    );
}

#[test]
fn test_audioconfig_validate_audio_ring_buffer_too_small() {
    // Audio ring buffer must be at least 2x render chunk size
    let config = AudioConfig {
        render_chunk_size: 256,
        audio_ring_buffer_size: 256,
        ..Default::default()
    };

    let result = config.validate();
    assert!(
        result.is_err(),
        "audio_ring_buffer_size too small should fail"
    );
    assert!(
        result
            .unwrap_err()
            .contains("audio_ring_buffer_size too small")
    );
}

#[test]
fn test_audioconfig_toml_serialization_roundtrip() {
    // Test that AudioConfig can be serialized to TOML and deserialized back
    let original = AudioConfig {
        cpal_buffer_size: 128,
        render_chunk_size: 512,
        audio_ring_buffer_size: 44100,
        message_ring_buffer_size: 2048,
        target_latency_ms: 100.0,
    };

    // Serialize to TOML
    let toml_string = toml::to_string(&original).expect("Failed to serialize AudioConfig");

    // Deserialize back
    let deserialized: AudioConfig =
        toml::from_str(&toml_string).expect("Failed to deserialize AudioConfig");

    // Verify values match
    assert_eq!(original.cpal_buffer_size, deserialized.cpal_buffer_size);
    assert_eq!(original.render_chunk_size, deserialized.render_chunk_size);
    assert_eq!(
        original.audio_ring_buffer_size,
        deserialized.audio_ring_buffer_size
    );
    assert_eq!(
        original.message_ring_buffer_size,
        deserialized.message_ring_buffer_size
    );
    assert_eq!(original.target_latency_ms, deserialized.target_latency_ms);
}

#[test]
fn test_audioconfig_toml_missing_fields_use_defaults() {
    // Test that missing fields in TOML use default values
    let toml_with_partial_config = r#"
        cpal_buffer_size = 128
        render_chunk_size = 512
    "#;

    let config: AudioConfig =
        toml::from_str(toml_with_partial_config).expect("Failed to parse partial TOML");

    // Specified fields should have the given values
    assert_eq!(config.cpal_buffer_size, 128);
    assert_eq!(config.render_chunk_size, 512);

    // Missing fields should have default values
    assert_eq!(config.audio_ring_buffer_size, 88200);
    assert_eq!(config.message_ring_buffer_size, 1024);
    assert_eq!(config.target_latency_ms, 50.0);
}

#[test]
fn test_audioconfig_load_from_file() {
    // Test loading AudioConfig from a temporary TOML file
    let temp_dir = std::env::temp_dir();
    let config_file = temp_dir.join("test_audio_config.toml");

    // Create a config file with custom values
    let toml_content = r#"
        cpal_buffer_size = 256
        render_chunk_size = 1024
        audio_ring_buffer_size = 176400
        message_ring_buffer_size = 512
        target_latency_ms = 25.0
    "#;

    {
        let mut file = std::fs::File::create(&config_file).expect("Failed to create temp file");
        file.write_all(toml_content.as_bytes())
            .expect("Failed to write to temp file");
    }

    // Load the config from file
    let file_content = std::fs::read_to_string(&config_file).expect("Failed to read temp file");
    let config: AudioConfig = toml::from_str(&file_content).expect("Failed to parse TOML file");

    // Verify values
    assert_eq!(config.cpal_buffer_size, 256);
    assert_eq!(config.render_chunk_size, 1024);
    assert_eq!(config.audio_ring_buffer_size, 176400);
    assert_eq!(config.message_ring_buffer_size, 512);
    assert_eq!(config.target_latency_ms, 25.0);

    // Clean up
    std::fs::remove_file(&config_file).expect("Failed to remove temp file");
}

// ============================================================================
// LogConfig Tests
// ============================================================================

#[test]
fn test_logconfig_default_values() {
    // Validate that LogConfig::default() returns the expected default values
    let config = LogConfig::default();

    assert_eq!(config.level, "info", "Default log level should be 'info'");
    assert!(!config.log_to_file, "Default log_to_file should be false");
    assert_eq!(
        config.log_file, "rustic.log",
        "Default log_file should be 'rustic.log'"
    );
    assert!(config.log_to_stdout, "Default log_to_stdout should be true");
}

#[test]
fn test_logconfig_toml_serialization_roundtrip() {
    // Test that LogConfig can be serialized to TOML and deserialized back
    let original = LogConfig {
        level: "debug".to_string(),
        log_to_file: true,
        log_file: "test.log".to_string(),
        log_to_stdout: false,
    };

    // Serialize to TOML
    let toml_string = toml::to_string(&original).expect("Failed to serialize LogConfig");

    // Deserialize back
    let deserialized: LogConfig =
        toml::from_str(&toml_string).expect("Failed to deserialize LogConfig");

    // Verify values match
    assert_eq!(original.level, deserialized.level);
    assert_eq!(original.log_to_file, deserialized.log_to_file);
    assert_eq!(original.log_file, deserialized.log_file);
    assert_eq!(original.log_to_stdout, deserialized.log_to_stdout);
}

#[test]
fn test_logconfig_toml_missing_fields_use_defaults() {
    // Test that missing fields in TOML use default values
    let toml_with_partial_config = r#"
        level = "warn"
    "#;

    let config: LogConfig =
        toml::from_str(toml_with_partial_config).expect("Failed to parse partial TOML");

    // Specified field should have the given value
    assert_eq!(config.level, "warn");

    // Missing fields should have default values
    assert!(!config.log_to_file);
    assert_eq!(config.log_file, "rustic.log");
    assert!(config.log_to_stdout);
}

#[test]
fn test_logconfig_load_from_file() {
    // Test loading LogConfig from a temporary TOML file
    let temp_dir = std::env::temp_dir();
    let config_file = temp_dir.join("test_log_config.toml");

    // Create a config file with custom values
    let toml_content = r#"
        level = "trace"
        log_to_file = true
        log_file = "custom.log"
        log_to_stdout = false
    "#;

    {
        let mut file = std::fs::File::create(&config_file).expect("Failed to create temp file");
        file.write_all(toml_content.as_bytes())
            .expect("Failed to write to temp file");
    }

    // Load the config from file
    let file_content = std::fs::read_to_string(&config_file).expect("Failed to read temp file");
    let config: LogConfig = toml::from_str(&file_content).expect("Failed to parse TOML file");

    // Verify values
    assert_eq!(config.level, "trace");
    assert!(config.log_to_file);
    assert_eq!(config.log_file, "custom.log");
    assert!(!config.log_to_stdout);

    // Clean up
    std::fs::remove_file(&config_file).expect("Failed to remove temp file");
}

#[test]
fn test_logconfig_all_log_levels() {
    // Test that all valid log levels can be deserialized
    let log_levels = ["trace", "debug", "info", "warn", "error"];

    for level in &log_levels {
        let toml_content = format!(r#"level = "{}""#, level);
        let config: LogConfig = toml::from_str(&toml_content)
            .unwrap_or_else(|_| panic!("Failed to parse log level: {}", level));

        assert_eq!(config.level, *level);
    }
}

// ============================================================================
// Combined Configuration Tests
// ============================================================================

#[test]
fn test_combined_config_toml() {
    // Test that AudioConfig and LogConfig can coexist in the same TOML structure
    #[derive(serde::Deserialize, serde::Serialize)]
    struct CombinedConfig {
        audio: AudioConfig,
        logging: LogConfig,
    }

    let toml_content = r#"
        [audio]
        cpal_buffer_size = 128
        render_chunk_size = 512

        [logging]
        level = "debug"
        log_to_file = true
    "#;

    let config: CombinedConfig =
        toml::from_str(toml_content).expect("Failed to parse combined config");

    // Verify audio config
    assert_eq!(config.audio.cpal_buffer_size, 128);
    assert_eq!(config.audio.render_chunk_size, 512);

    // Verify logging config
    assert_eq!(config.logging.level, "debug");
    assert!(config.logging.log_to_file);
}
