use crate::app::filesystem::FSConfig;
use crate::app::prelude::SystemConfig;
use log::error;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::error::AppError;

#[derive(Debug, Default, Deserialize, Serialize)]
/// The application configuration
pub struct AppConfig {
    pub fs: FSConfig,
    pub system: SystemConfig,

    #[serde(default)]
    pub audio: crate::audio::AudioConfig,

    #[serde(default)]
    pub logging: crate::audio::LogConfig,
}

impl AppConfig {
    pub fn from_file(file: &Path) -> Result<Self, AppError> {
        let root_path = FSConfig::app_root_dir().unwrap_or_else(|e| {
            error!("Unable to build app root dir: {}", e);
            PathBuf::from("./")
        });

        let config_file = root_path.join(file);

        let config = if config_file.exists() {
            toml::from_str(&std::fs::read_to_string(config_file)?).unwrap_or_else(|e| {
                error!("Unable to parse config file: {}", e);
                AppConfig::default()
            })
        } else {
            return Err(AppError::FileNotFound);
        };

        Ok(config)
    }
}
