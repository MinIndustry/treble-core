use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MixCommand {
    VolumeUp(u8),
    VolumeDown(u8),
    SetVolume(f32, u8),
    Mute(u8),
    MuteAll,
}
