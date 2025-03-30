use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct SystemConfig {
    pub sample_rate: f32,
    pub master_volume: f32,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            sample_rate: 44100.0,
            master_volume: 1.0,
        }
    }
}
