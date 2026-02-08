use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SettingsCommand {
    SwitchKeyboardLayout(String),
    ToggleHelp,
    Undo,
    Redo,
    TakeSnapshot,
    RestoreSnapshot(usize),
    LinkAll,
    UnlinkAll,
    SwapRows,
    CopyRowSettings(u8, u8),
    ToggleMetronome,
    SetTempo(u32),
    TempoUp,
    TempoDown,
    StartSessionRecording,
    StopSessionRecording,
    PlaySession,
    StopSession,
    SaveSession(String),
    LoadSession(String),
    ListOutputDevices,
    SelectOutputDevice(String),
}
