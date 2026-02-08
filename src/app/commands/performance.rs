use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PerformanceCommand {
    PitchBendUp(f32, u8),
    PitchBendDown(f32, u8),
    PitchBendReset(u8),
    Vibrato(f32, u8),
    Tremolo(f32, u8),
}
