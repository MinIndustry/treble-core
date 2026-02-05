use serde::{Deserialize, Serialize};

use super::prelude::*;

/// Commands for a keyboard-based music application
///
/// This enum represents all possible commands that can be issued through
/// keyboard input in the application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Commands {
    /// Quit the application
    Quit,
    /// Reset the application state
    Reset,

    // Note playing commands
    /// Start playing a note with specified parameters
    /// Parameters: note_value (0-11 for C through B), instrument_row (0 or 1), velocity (0.0-1.0)
    NoteStart(u8, u8, f32),
    /// Stop playing a note
    /// Parameters: note_value (0-11 for C through B), instrument_row (0 or 1)
    NoteStop(u8, u8),

    // Octave control
    /// Increase the octave for a specific instrument row
    OctaveUp(u8), // instrument_row
    /// Decrease the octave for a specific instrument row
    OctaveDown(u8), // instrument_row
    /// Set a specific octave (0-8) for a specific instrument row
    SetOctave(u8, u8), // octave, instrument_row
    /// Link the octaves of both instrument rows so they change together
    LinkOctaves,
    /// Unlink the octaves of instrument rows to control them separately
    UnlinkOctaves,

    // Looping functionality
    /// Start recording a loop
    StartRecording,
    /// Stop recording and save the loop
    StopRecording,
    /// Play the recorded loop
    PlayLoop,
    /// Stop the playing loop
    StopLoop,
    /// Clear the recorded loop
    ClearLoop,
    /// Set the loop to repeat
    LoopRepeat(bool),
    /// Set the number of times to repeat the loop
    LoopRepeatCount(u32),

    // Numeric key loop storage/retrieval
    /// Save the current loop to a numbered slot (0-9)
    /// Parameters: slot_number, instrument_row (0 or 1)
    SaveLoopToSlot(u8, u8),
    /// Load a loop from a numbered slot (0-9)
    /// Parameters: slot_number, instrument_row (0 or 1)
    LoadLoopFromSlot(u8, u8),
    /// Clear a loop from a numbered slot (0-9)
    /// Parameter: slot_number
    ClearLoopSlot(u8),
    /// Toggle quick swap between two loop slots
    /// Parameters: slot_number_1, slot_number_2
    ToggleLoopSlots(u8, u8),

    // Performance modifiers
    /// Bend the pitch up for a specific instrument row
    /// Parameters: amount, instrument_row (0 or 1)
    PitchBendUp(f32, u8),
    /// Bend the pitch down for a specific instrument row
    /// Parameters: amount, instrument_row (0 or 1)
    PitchBendDown(f32, u8),
    /// Reset pitch bend to normal for a specific instrument row
    /// Parameter: instrument_row (0 or 1)
    PitchBendReset(u8),
    /// Apply vibrato effect for a specific instrument row
    /// Parameters: amount, instrument_row (0 or 1)
    Vibrato(f32, u8),
    /// Apply tremolo effect for a specific instrument row
    /// Parameters: amount, instrument_row (0 or 1)
    Tremolo(f32, u8),

    // Volume controls
    /// Increase volume for a specific instrument row
    /// Parameter: instrument_row (0 or 1)
    VolumeUp(u8),
    /// Decrease volume for a specific instrument row
    /// Parameter: instrument_row (0 or 1)
    VolumeDown(u8),
    /// Set the volume to a specific level (0.0 - 1.0) for a specific instrument row
    /// Parameters: level, instrument_row (0 or 1)
    SetVolume(f32, u8),
    /// Mute a specific instrument row
    /// Parameter: instrument_row (0 or 1)
    Mute(u8),
    /// Mute all sound (both instrument rows)
    MuteAll,

    // Instrument selection
    /// Change to a different instrument for a specific row
    /// Parameters: instrument_index, instrument_row (0 or 1)
    SelectInstrument(usize, u8),
    /// Cycle to the next instrument for a specific row
    /// Parameter: instrument_row (0 or 1)
    NextInstrument(u8),
    /// Cycle to the previous instrument for a specific row
    /// Parameter: instrument_row (0 or 1)
    PreviousInstrument(u8),
    /// Link the instruments of both rows so they change together
    LinkInstruments,
    /// Unlink the instruments to control them separately
    UnlinkInstruments,

    // Recording and playback
    /// Start recording a session
    StartSessionRecording,
    /// Stop recording a session
    StopSessionRecording,
    /// Play back the recorded session
    PlaySession,
    /// Stop the session playback
    StopSession,
    /// Save the current session to a file
    SaveSession(String),
    /// Load a session from a file
    LoadSession(String),

    // Metronome controls
    /// Toggle the metronome on/off
    ToggleMetronome,
    /// Set the metronome tempo (BPM)
    SetTempo(u32),
    /// Increase the tempo
    TempoUp,
    /// Decrease the tempo
    TempoDown,

    // Mix and effects
    /// Apply a reverb effect for a specific instrument row
    /// Parameters: amount, instrument_row (0 or 1)
    Reverb(f32, u8),
    /// Apply a delay effect for a specific instrument row
    /// Parameters: time, feedback, instrument_row (0 or 1)
    Delay(f32, f32, u8),
    /// Apply a chorus effect for a specific instrument row
    /// Parameters: amount, instrument_row (0 or 1)
    Chorus(f32, u8),
    /// Apply a filter for a specific instrument row
    /// Parameters: cutoff frequency, resonance, instrument_row (0 or 1)
    Filter(f32, f32, u8),
    /// Toggle distortion effect for a specific instrument row
    /// Parameter: instrument_row (0 or 1)
    ToggleDistortion(u8),

    // Keyboard layout
    /// Switch keyboard layout (e.g., piano, chromatic, etc.)
    SwitchKeyboardLayout(String),

    // Utility commands
    /// Toggle help display
    ToggleHelp,
    /// Undo the last action
    Undo,
    /// Redo the last undone action
    Redo,
    /// Take a snapshot of the current state
    TakeSnapshot,
    /// Restore from a previous snapshot
    RestoreSnapshot(usize),

    // Row synchronization controls
    /// Link all parameters (octaves, instruments, effects) between rows
    LinkAll,
    /// Unlink all parameters between rows
    UnlinkAll,
    /// Swap the settings between the two instrument rows
    SwapRows,
    /// Copy settings from one row to the other
    /// Parameters: source_row, destination_row
    CopyRowSettings(u8, u8),
}

impl Commands {
    /// Validate a command against the current app state
    pub fn validate(&self, _app: &App) -> Result<(), crate::audio::CommandError> {
        use crate::audio::CommandError;

        match self {
            Commands::NoteStart(_, row, velocity) => {
                if *row >= 2 {
                    return Err(CommandError::RowOutOfBounds(*row));
                }
                if *velocity < 0.0 || *velocity > 1.0 {
                    return Err(CommandError::InvalidVolume(*velocity));
                }
                Ok(())
            }
            Commands::NoteStop(_, row) => {
                if *row >= 2 {
                    return Err(CommandError::RowOutOfBounds(*row));
                }
                Ok(())
            }
            Commands::SetOctave(octave, row) => {
                if *row >= 2 {
                    return Err(CommandError::RowOutOfBounds(*row));
                }
                if *octave > 8 {
                    return Err(CommandError::InvalidOctave(*octave));
                }
                Ok(())
            }
            Commands::OctaveUp(row) | Commands::OctaveDown(row) => {
                if *row >= 2 {
                    return Err(CommandError::RowOutOfBounds(*row));
                }
                Ok(())
            }
            // Most commands don't need validation
            _ => Ok(()),
        }
    }

    /// Translate a command to an audio message for the audio render thread
    pub fn translate_to_audio_message(&self, app: &mut App) -> Option<crate::audio::AudioMessage> {
        use crate::audio::AudioMessage;

        match self {
            Commands::NoteStart(note, row, velocity) => {
                let note = app.rows[*row as usize].get_note(*note);
                let instrument_idx = app.rows[*row as usize].instrument;
                Some(AudioMessage::NoteStart {
                    instrument_idx,
                    note,
                    velocity: *velocity,
                })
            }
            Commands::NoteStop(note, row) => {
                let note = app.rows[*row as usize].get_note(*note);
                let instrument_idx = app.rows[*row as usize].instrument;
                Some(AudioMessage::NoteStop {
                    instrument_idx,
                    note,
                })
            }
            Commands::OctaveUp(row) | Commands::OctaveDown(row) | Commands::SetOctave(_, row) => {
                // Octave changes are handled in the command thread
                Some(AudioMessage::SetOctave {
                    row: *row as usize,
                    octave: app.rows[*row as usize].octave,
                })
            }
            Commands::Quit => Some(AudioMessage::Shutdown),
            // Most commands don't directly affect audio rendering
            _ => None,
        }
    }
}
