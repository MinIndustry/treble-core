//! Audio Architecture Tests
//!
//! This test suite validates the core audio system components:
//! - SharedAudioState initialization and behavior
//! - BackendEvent creation and handling
//! - AudioMessage creation and cloning
//! - Atomic operations on shared state
//!
//! Note: Full integration tests with AudioHandle.shutdown() would require
//! spawning actual audio threads, which is complex for unit testing.
//! These tests focus on the data structures and their behavior.

use rustic::Note;
use rustic::audio::{AudioMessage, BackendEvent, SharedAudioState};
use rustic::core::utils::NOTES;
use std::sync::atomic::Ordering;

// ============================================================================
// SharedAudioState Tests
// ============================================================================

#[test]
fn test_shared_audio_state_initialization() {
    // SharedAudioState should initialize with correct default values
    let state = SharedAudioState::new();

    assert!(
        !state.shutdown.load(Ordering::Relaxed),
        "Initial shutdown flag should be false"
    );
    assert_eq!(
        state.buffer_underruns.load(Ordering::Relaxed),
        0,
        "Initial buffer_underruns should be 0"
    );
    assert_eq!(
        state.sample_rate.load(Ordering::Relaxed),
        44100,
        "Initial sample_rate should be 44100"
    );
    assert_eq!(
        state.master_volume.load(Ordering::Relaxed),
        1.0,
        "Initial master_volume should be 1.0"
    );
}

#[test]
fn test_shared_audio_state_default() {
    // Default trait should produce same values as new()
    let state = SharedAudioState::default();

    assert!(
        !state.shutdown.load(Ordering::Relaxed),
        "Default shutdown flag should be false"
    );
    assert_eq!(
        state.buffer_underruns.load(Ordering::Relaxed),
        0,
        "Default buffer_underruns should be 0"
    );
    assert_eq!(
        state.sample_rate.load(Ordering::Relaxed),
        44100,
        "Default sample_rate should be 44100"
    );
    assert_eq!(
        state.master_volume.load(Ordering::Relaxed),
        1.0,
        "Default master_volume should be 1.0"
    );
}

#[test]
fn test_shared_audio_state_shutdown_flag() {
    // Test setting and reading the shutdown flag
    let state = SharedAudioState::new();

    // Initially false
    assert!(!state.shutdown.load(Ordering::Relaxed));

    // Set to true
    state.shutdown.store(true, Ordering::Release);
    assert!(
        state.shutdown.load(Ordering::Acquire),
        "Shutdown flag should be set to true"
    );

    // Set back to false
    state.shutdown.store(false, Ordering::Release);
    assert!(
        !state.shutdown.load(Ordering::Acquire),
        "Shutdown flag should be reset to false"
    );
}

#[test]
fn test_shared_audio_state_buffer_underruns() {
    // Test incrementing buffer underrun counter
    let state = SharedAudioState::new();

    // Start at 0
    assert_eq!(state.buffer_underruns.load(Ordering::Relaxed), 0);

    // Increment
    state.buffer_underruns.fetch_add(1, Ordering::Relaxed);
    assert_eq!(
        state.buffer_underruns.load(Ordering::Relaxed),
        1,
        "Buffer underruns should increment"
    );

    // Increment multiple times
    for _ in 0..5 {
        state.buffer_underruns.fetch_add(1, Ordering::Relaxed);
    }
    assert_eq!(
        state.buffer_underruns.load(Ordering::Relaxed),
        6,
        "Buffer underruns should accumulate"
    );
}

#[test]
fn test_shared_audio_state_sample_rate() {
    // Test setting and reading sample rate
    let state = SharedAudioState::new();

    // Test common sample rates
    let sample_rates = [22050, 44100, 48000, 88200, 96000, 192000];

    for rate in &sample_rates {
        state.sample_rate.store(*rate, Ordering::Relaxed);
        assert_eq!(
            state.sample_rate.load(Ordering::Relaxed),
            *rate,
            "Sample rate should be set to {}",
            rate
        );
    }
}

#[test]
fn test_shared_audio_state_master_volume() {
    // Test setting and reading master volume
    let state = SharedAudioState::new();

    // Test various volume levels
    let volumes = [0.0, 0.25, 0.5, 0.75, 1.0, 1.5];

    for volume in &volumes {
        state.master_volume.store(*volume, Ordering::Relaxed);
        let loaded = state.master_volume.load(Ordering::Relaxed);
        assert!(
            (loaded - volume).abs() < 0.0001,
            "Master volume should be set to {}",
            volume
        );
    }
}

// ============================================================================
// BackendEvent Tests
// ============================================================================

#[test]
fn test_backend_event_audio_started() {
    // Test creating AudioStarted event
    let event = BackendEvent::AudioStarted { sample_rate: 48000 };

    match event {
        BackendEvent::AudioStarted { sample_rate } => {
            assert_eq!(sample_rate, 48000, "Sample rate should be 48000");
        }
        _ => panic!("Expected AudioStarted event"),
    }
}

#[test]
fn test_backend_event_audio_stopped() {
    // Test creating AudioStopped event
    let event = BackendEvent::AudioStopped;

    match event {
        BackendEvent::AudioStopped => {
            // Expected
        }
        _ => panic!("Expected AudioStopped event"),
    }
}

#[test]
fn test_backend_event_command_error() {
    // Test creating CommandError event
    let event = BackendEvent::CommandError {
        command: "NoteStart".to_string(),
        error: "Invalid velocity".to_string(),
    };

    match event {
        BackendEvent::CommandError { command, error } => {
            assert_eq!(command, "NoteStart", "Command should be 'NoteStart'");
            assert_eq!(error, "Invalid velocity", "Error should match");
        }
        _ => panic!("Expected CommandError event"),
    }
}

#[test]
fn test_backend_event_buffer_underrun() {
    // Test creating BufferUnderrun event
    let event = BackendEvent::BufferUnderrun { count: 42 };

    match event {
        BackendEvent::BufferUnderrun { count } => {
            assert_eq!(count, 42, "Underrun count should be 42");
        }
        _ => panic!("Expected BufferUnderrun event"),
    }
}

#[test]
fn test_backend_event_metrics() {
    // Test creating Metrics event
    let event = BackendEvent::Metrics {
        cpu_usage: 25.5,
        latency_ms: 12.3,
    };

    match event {
        BackendEvent::Metrics {
            cpu_usage,
            latency_ms,
        } => {
            assert!((cpu_usage - 25.5).abs() < 0.0001, "CPU usage should match");
            assert!((latency_ms - 12.3).abs() < 0.0001, "Latency should match");
        }
        _ => panic!("Expected Metrics event"),
    }
}

#[test]
fn test_backend_event_clone() {
    // Test that BackendEvent can be cloned
    let original = BackendEvent::AudioStarted { sample_rate: 44100 };
    let cloned = original.clone();

    match (original, cloned) {
        (
            BackendEvent::AudioStarted { sample_rate: rate1 },
            BackendEvent::AudioStarted { sample_rate: rate2 },
        ) => {
            assert_eq!(rate1, rate2, "Cloned event should have same sample rate");
        }
        _ => panic!("Both should be AudioStarted events"),
    }
}

#[test]
fn test_backend_event_debug() {
    // Test that BackendEvent can be formatted with Debug
    let event = BackendEvent::AudioStarted { sample_rate: 48000 };
    let debug_str = format!("{:?}", event);

    assert!(
        debug_str.contains("AudioStarted"),
        "Debug output should contain event type"
    );
    assert!(
        debug_str.contains("48000"),
        "Debug output should contain sample rate"
    );
}

// ============================================================================
// AudioMessage Tests
// ============================================================================

#[test]
fn test_audiomessage_notestart() {
    // Test creating NoteStart message
    let msg = AudioMessage::NoteStart {
        instrument_idx: 0,
        note: Note(NOTES::A, 4),
        velocity: 0.8,
    };

    match msg {
        AudioMessage::NoteStart {
            instrument_idx,
            note,
            velocity,
        } => {
            assert_eq!(instrument_idx, 0, "Instrument index should be 0");
            assert_eq!(note, Note(NOTES::A, 4), "Note should be A4");
            assert_eq!(velocity, 0.8, "Velocity should be 0.8");
        }
        _ => panic!("Expected NoteStart message"),
    }
}

#[test]
fn test_audiomessage_notestop() {
    // Test creating NoteStop message
    let msg = AudioMessage::NoteStop {
        instrument_idx: 1,
        note: Note(NOTES::C, 5),
    };

    match msg {
        AudioMessage::NoteStop {
            instrument_idx,
            note,
        } => {
            assert_eq!(instrument_idx, 1, "Instrument index should be 1");
            assert_eq!(note, Note(NOTES::C, 5), "Note should be C5");
        }
        _ => panic!("Expected NoteStop message"),
    }
}

#[test]
fn test_audiomessage_setoctave() {
    // Test creating SetOctave message
    let msg = AudioMessage::SetOctave { row: 0, octave: 6 };

    match msg {
        AudioMessage::SetOctave { row, octave } => {
            assert_eq!(row, 0, "Row should be 0");
            assert_eq!(octave, 6, "Octave should be 6");
        }
        _ => panic!("Expected SetOctave message"),
    }
}

#[test]
fn test_audiomessage_setmastervolume() {
    // Test creating SetMasterVolume message
    let msg = AudioMessage::SetMasterVolume { volume: 0.7 };

    match msg {
        AudioMessage::SetMasterVolume { volume } => {
            assert_eq!(volume, 0.7, "Volume should be 0.7");
        }
        _ => panic!("Expected SetMasterVolume message"),
    }
}

#[test]
fn test_audiomessage_setsamplerate() {
    // Test creating SetSampleRate message
    let msg = AudioMessage::SetSampleRate { rate: 48000 };

    match msg {
        AudioMessage::SetSampleRate { rate } => {
            assert_eq!(rate, 48000, "Rate should be 48000");
        }
        _ => panic!("Expected SetSampleRate message"),
    }
}

#[test]
fn test_audiomessage_shutdown() {
    // Test creating Shutdown message
    let msg = AudioMessage::Shutdown;

    match msg {
        AudioMessage::Shutdown => {
            // Expected
        }
        _ => panic!("Expected Shutdown message"),
    }
}

#[test]
fn test_audiomessage_clone() {
    // Test that AudioMessage can be cloned
    let original = AudioMessage::NoteStart {
        instrument_idx: 0,
        note: Note(NOTES::D, 3),
        velocity: 0.5,
    };
    let cloned = original.clone();

    match (original, cloned) {
        (
            AudioMessage::NoteStart {
                instrument_idx: idx1,
                note: note1,
                velocity: vel1,
            },
            AudioMessage::NoteStart {
                instrument_idx: idx2,
                note: note2,
                velocity: vel2,
            },
        ) => {
            assert_eq!(idx1, idx2, "Cloned instrument index should match");
            assert_eq!(note1, note2, "Cloned note should match");
            assert_eq!(vel1, vel2, "Cloned velocity should match");
        }
        _ => panic!("Both should be NoteStart messages"),
    }
}

#[test]
fn test_audiomessage_debug() {
    // Test that AudioMessage can be formatted with Debug
    let msg = AudioMessage::NoteStart {
        instrument_idx: 0,
        note: Note(NOTES::E, 4),
        velocity: 0.9,
    };
    let debug_str = format!("{:?}", msg);

    assert!(
        debug_str.contains("NoteStart"),
        "Debug output should contain message type"
    );
}

#[test]
fn test_audiomessage_all_variants() {
    // Test that all AudioMessage variants can be created
    let messages = vec![
        AudioMessage::NoteStart {
            instrument_idx: 0,
            note: Note(NOTES::C, 4),
            velocity: 0.8,
        },
        AudioMessage::NoteStop {
            instrument_idx: 0,
            note: Note(NOTES::C, 4),
        },
        AudioMessage::SetOctave { row: 0, octave: 5 },
        AudioMessage::SetMasterVolume { volume: 0.75 },
        AudioMessage::SetSampleRate { rate: 44100 },
        AudioMessage::Shutdown,
    ];

    // Verify all can be cloned and debugged
    for msg in messages {
        let cloned = msg.clone();
        let _ = format!("{:?}", msg);
        let _ = format!("{:?}", cloned);
    }
}

// ============================================================================
// Thread Safety Tests (Conceptual)
// ============================================================================

#[test]
fn test_shared_audio_state_send_sync() {
    // This test verifies at compile time that SharedAudioState is Send + Sync
    // which is required for sharing between threads
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<SharedAudioState>();
    assert_sync::<SharedAudioState>();
}

#[test]
fn test_audiomessage_send() {
    // Verify AudioMessage can be sent between threads
    fn assert_send<T: Send>() {}

    assert_send::<AudioMessage>();
}

#[test]
fn test_backend_event_send() {
    // Verify BackendEvent can be sent between threads
    fn assert_send<T: Send>() {}

    assert_send::<BackendEvent>();
}

// ============================================================================
// Integration-style Tests (Limited Scope)
// ============================================================================

#[test]
fn test_shared_state_concurrent_access_pattern() {
    // Simulate the pattern of concurrent access without actual threads
    // This demonstrates the intended usage pattern
    let state = SharedAudioState::new();

    // Simulate command thread updating sample rate
    state.sample_rate.store(48000, Ordering::Release);

    // Simulate render thread reading sample rate
    let rate = state.sample_rate.load(Ordering::Acquire);
    assert_eq!(rate, 48000);

    // Simulate callback reporting underrun
    state.buffer_underruns.fetch_add(1, Ordering::Relaxed);

    // Simulate main thread reading metrics
    let underruns = state.buffer_underruns.load(Ordering::Relaxed);
    assert_eq!(underruns, 1);

    // Simulate shutdown signal
    state.shutdown.store(true, Ordering::Release);
    let should_shutdown = state.shutdown.load(Ordering::Acquire);
    assert!(should_shutdown);
}

#[test]
fn test_message_queue_pattern() {
    // Demonstrate the pattern of queuing audio messages
    // (This doesn't test the actual queue, just the message types)
    let messages = vec![
        AudioMessage::NoteStart {
            instrument_idx: 0,
            note: Note(NOTES::C, 4),
            velocity: 0.8,
        },
        AudioMessage::SetOctave { row: 0, octave: 5 },
        AudioMessage::NoteStop {
            instrument_idx: 0,
            note: Note(NOTES::C, 4),
        },
    ];

    // Verify we can process them
    assert_eq!(messages.len(), 3, "Should have 3 messages queued");

    for msg in messages {
        match msg {
            AudioMessage::NoteStart { .. } => {
                // Would trigger note start in real system
            }
            AudioMessage::SetOctave { .. } => {
                // Would update octave in real system
            }
            AudioMessage::NoteStop { .. } => {
                // Would trigger note stop in real system
            }
            _ => {}
        }
    }
}
