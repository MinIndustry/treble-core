use log::error;
use serde::{Deserialize, Serialize};
use std::{default::Default, path::PathBuf};

use crate::{fs::app_root_dir, inputs::InputConfig};

use super::system::SystemConfig;

#[derive(Debug, Deserialize, Serialize)]
/// Application Filesystem configuration
pub struct FSConfig {
    pub score_path: PathBuf,
    pub instrument_path: PathBuf,
    pub recordings_path: PathBuf,
}

impl Default for FSConfig {
    fn default() -> FSConfig {
        let root_path = match app_root_dir() {
            Ok(path) => path,
            Err(e) => {
                error!("Unable to build app root dir: {}", e);
                PathBuf::from("./")
            }
        };
        // TODO: Implement actual path building
        FSConfig {
            score_path: root_path.join("scores/"),
            instrument_path: root_path.join("instruments/"),
            recordings_path: root_path.join("recordings/"),
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
/// The application configuration
pub struct AppConfig {
    pub input: InputConfig,
    pub fs: FSConfig,
    pub system: SystemConfig,
    // pub sound_system: SoundSystemConfig,
}
