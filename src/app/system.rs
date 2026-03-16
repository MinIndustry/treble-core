//! The System module is used to interact with the system.
//! Its purpose is to help organize the system and provide a way to interact with it.
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemConfig {
    pub sample_rate: u32,
    pub master_volume: f32,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            master_volume: 1.0,
        }
    }
}
