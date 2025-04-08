use serde::{Deserialize, Serialize};

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
        let root_path = match Self::app_root_dir() {
            Ok(path) => path,
            Err(e) => {
                error!("Unable to build app root dir: {}", e);
                PathBuf::from("./")
            }
        };

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
    fn debug_dir_check() -> Result<PathBuf, ()> {
        let out_dir = Path::new("./.dist/");
        if !out_dir.exists() {
            if let Err(e) = fs::create_dir(out_dir) {
                error!("Failed to create output directory: {}", e);
                return Err(());
            }
        }
        Ok(out_dir.to_path_buf())
    }

    /// Builds the debug direction as given
    ///
    /// Returns an empty result
    fn debug_dir_build(path: &Path) -> Result<(), ()> {
        if !path.exists() {
            if let Err(e) = fs::create_dir(path) {
                error!("Failed to create output directory: {}", e);
                return Err(());
            }
        }
        Ok(())
    }

    /// Builds the required path to save the file from the given module with the given name
    ///
    /// Returns a result containing the built path with the file name.
    pub fn debug_dir(module: &str, filename: &str) -> Result<PathBuf, ()> {
        let base_path = Self::debug_dir_check()?;
        let full_path = base_path.join(module);
        Self::debug_dir_build(&full_path)?;
        Ok(full_path.join(filename))
    }

    /// Builds the required path to save the file from the given module with the given name.
    /// Adds a timestamp to allow for time-differentiation of the saved files.
    ///
    /// Returns a result containing the built path with the file name.
    pub fn _stamped_debug_dir(module: &str, filename: &str) -> Result<PathBuf, ()> {
        let base_path = Self::debug_dir_check()?;
        let full_path = base_path.join(module);
        let timestamp = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S").to_string();
        let full_path = full_path.join(format!("{}_{}", timestamp, filename));
        Ok(full_path)
    }

    /// Returns the app's default root path for saving configuration files & other.
    /// This is supposed to be used if the application's settings structure contains
    /// no information about the path.
    pub fn app_root_dir() -> Result<PathBuf, String> {
        use directories::ProjectDirs;
        let root_path = ProjectDirs::from(crate::APP_ID.2, crate::APP_ID.1, crate::APP_ID.0)
            .and_then(|d| Some(d.config_dir().to_path_buf()))
            .ok_or("Unable to build app's configuration direction".to_string())?;

        if !root_path.exists() {
            fs::create_dir(&root_path).map_err(|e| e.to_string())?
        }

        Ok(root_path)
    }
}
