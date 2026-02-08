use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoopCommand {
    StartRecording,
    StopRecording,
    PlayLoop,
    StopLoop,
    ClearLoop,
    LoopRepeat(bool),
    LoopRepeatCount(u32),
    SaveLoopToSlot(u8, u8),
    LoadLoopFromSlot(u8, u8),
    ClearLoopSlot(u8),
    ToggleLoopSlots(u8, u8),
}
