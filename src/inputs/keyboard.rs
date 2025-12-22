//! Keyboard input handling for Linux systems
//!
//! This module provides functionality to detect and handle keyboard input
//! events from hardware keyboards using the evdev interface.

#[cfg(feature = "input")]
use evdev::{Device, EventType, Key};
#[cfg(feature = "input")]
use log::{info, warn};
#[cfg(feature = "input")]
use std::path::Path;

#[cfg(feature = "input")]
/// Find the first available keyboard device
pub fn find_keyboard() -> Option<Device> {
    let devices_dir = Path::new("/dev/input");

    if !devices_dir.exists() {
        warn!("Input devices directory not found: /dev/input");
        return None;
    }

    // Try to find keyboard devices
    for entry in std::fs::read_dir(devices_dir).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();

        if let Some(file_name) = path.file_name() {
            if let Some(name) = file_name.to_str() {
                if name.starts_with("event") {
                    if let Ok(device) = Device::open(&path) {
                        // Check if this device has keyboard capabilities
                        if device.supported_events().contains(EventType::KEY) {
                            if let Some(keys) = device.supported_keys() {
                                // Check for common keyboard keys
                                if keys.contains(Key::KEY_Q) && keys.contains(Key::KEY_W) {
                                    info!("Found keyboard device: {}", path.display());
                                    return Some(device);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    warn!("No keyboard devices found");
    None
}

#[cfg(not(feature = "input"))]
/// Stub function that returns None when input features are disabled
pub fn find_keyboard() -> Option<()> {
    None
}
