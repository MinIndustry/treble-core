//! The filesystem module is used to interact with the filesystem.
//! Its purpose is to help organize the filesystem and provide a way to interact with it.
use serde::{Deserialize, Serialize};

use crate::app::error::AppError;
use log::error;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Serialize)]
/// Application Filesystem configuration
pub struct FSConfig {
    pub root_dir: PathBuf,
    pub score_path: PathBuf,
    pub instrument_path: PathBuf,
    pub recordings_path: PathBuf,
}

impl Default for FSConfig {
    fn default() -> FSConfig {
        let root_path = Self::app_root_dir().unwrap_or_else(|e| {
            error!("Unable to build app root dir: {}", e);
            PathBuf::from("./")
        });

        // TODO: Implement actual path building
        FSConfig {
            root_dir: root_path.clone(),
            score_path: root_path.join("scores/"),
            instrument_path: root_path.join("instruments/"),
            recordings_path: root_path.join("recordings/"),
        }
    }
}

impl FSConfig {
    /// Verifies the existence of the debug dir (usually `PWD/.dist`)
    /// and builds it if not existing.
    ///
    /// Returns a result with the debug
    fn debug_dir_check() -> Result<PathBuf, AppError> {
        let out_dir = Path::new("./.dist/");
        if !out_dir.exists() {
            fs::create_dir(out_dir)?;
        }
        Ok(out_dir.to_path_buf())
    }

    /// Builds the debug direction as given
    ///
    /// Returns an empty result
    fn debug_dir_build(path: &Path) -> Result<(), AppError> {
        if !path.exists() {
            fs::create_dir(path)?;
        }
        Ok(())
    }

    /// Builds the required path to save the file from the given module with the given name
    ///
    /// Returns a result containing the built path with the file name.
    #[allow(clippy::result_unit_err)]
    pub fn debug_dir(module: &str, filename: &str) -> Result<PathBuf, AppError> {
        let base_path = Self::debug_dir_check()?;
        let full_path = base_path.join(module);
        Self::debug_dir_build(&full_path)?;
        Ok(full_path.join(filename))
    }

    /// Builds the required path to save the file from the given module with the given name.
    /// Adds a timestamp to allow for time-differentiation of the saved files.
    ///
    /// Returns a result containing the built path with the file name.
    pub fn _stamped_debug_dir(module: &str, filename: &str) -> Result<PathBuf, AppError> {
        let base_path = Self::debug_dir_check()?;
        let full_path = base_path.join(module);
        let timestamp = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S").to_string();
        let full_path = full_path.join(format!("{}_{}", timestamp, filename));
        Ok(full_path)
    }

    /// Returns the app's default root path for saving configuration files & other.
    /// This is supposed to be used if the application's settings structure contains
    /// no information about the path.
    pub fn app_root_dir() -> Result<PathBuf, AppError> {
        use directories::ProjectDirs;
        let root_path = ProjectDirs::from(crate::APP_ID.2, crate::APP_ID.1, crate::APP_ID.0)
            .map(|d| d.config_dir().to_path_buf())
            .ok_or(AppError::ConfigDirError)?;

        if !root_path.exists() {
            fs::create_dir(&root_path)?
        }

        Ok(root_path)
    }
}
