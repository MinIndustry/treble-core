use crate::app::filesystem::FSConfig;
use crate::app::prelude::SystemConfig;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::error::AppError;

#[derive(Debug, Default, Deserialize, Serialize)]
/// The application configuration
pub struct AppConfig {
    #[serde(default)]
    pub fs: FSConfig,

    #[serde(default)]
    pub system: SystemConfig,

    #[serde(default)]
    pub audio: crate::audio::AudioConfig,

    #[serde(default)]
    pub logging: crate::audio::LogConfig,
}

impl AppConfig {
    /// Load config from a file path. If the path is relative, it is resolved
    /// against the app root directory (`~/.config/rustic/`).
    pub fn from_file(file: &Path) -> Result<Self, AppError> {
        let config_file = if file.is_absolute() {
            file.to_path_buf()
        } else {
            let root_path = FSConfig::app_root_dir().unwrap_or_else(|e| {
                log::warn!("Unable to build app root dir: {}", e);
                PathBuf::from("./")
            });
            root_path.join(file)
        };

        if !config_file.exists() {
            return Err(AppError::FileNotFound);
        }

        let contents = std::fs::read_to_string(&config_file)?;
        let config =
            toml::from_str(&contents).map_err(|e| AppError::ConfigParseError(e.to_string()))?;

        Ok(config)
    }
}
