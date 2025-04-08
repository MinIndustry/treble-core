use serde::{Deserialize, Serialize};
use std::default::Default;

#[cfg(feature = "linux")]
pub mod keyboard;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct InputConfig {
    device_path: Option<String>,
}

impl InputConfig {
    pub fn new() -> Self {
        InputConfig { device_path: None }
    }

    pub fn get_device_path(&self) -> Option<String> {
        self.device_path.clone()
    }

    pub fn set_device_path(&mut self, path: String) {
        self.device_path = Some(path);
    }

    /// Best effort to guess the device path. This function
    /// currently only works on Linux.
    pub fn guess_device_path(&mut self) {
        #[cfg(feature = "linux")]
        {
            if let Some(keyboard) = keyboard::find_keyboard() {
                if let Some(path) = keyboard.physical_path() {
                    self.set_device_path(path.to_string());
                } else {
                    warn!("Unable to find a physical path for the keyboard device.");
                }
            } else {
                warn!("Unable to find a keyboard device.")
            }
        }
    }
}
