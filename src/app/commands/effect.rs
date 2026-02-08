use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EffectCommand {
    Reverb(f32, u8),
    Delay(f32, f32, u8),
    Chorus(f32, u8),
    Filter(f32, f32, u8),
    ToggleDistortion(u8),
}
