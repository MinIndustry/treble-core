use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::default::Default;
use std::sync::mpsc::{self, Receiver, Sender};

// Sub-modules
pub mod commands;
mod input;

// Platform-specific modules
#[cfg(all(feature = "linux", target_os = "linux"))]
mod linux;

#[cfg(all(feature = "windows", target_os = "windows"))]
mod windows;

#[cfg(all(feature = "macos", target_os = "macos"))]
mod macos;

// Legacy code, kept for compatibility
#[cfg(feature = "linux")]
pub mod keyboard;

// Re-export key types from the input module
pub use input::{InputError, InputEvent, KeyAction, KeyCode, Modifiers};

/// Input configuration
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct InputConfig {
    device_path: Option<String>,
    /// Key-to-command mapping
    #[serde(skip)]
    key_mapping: HashMap<KeyMappingEntry, commands::Commands>,
}

/// Entry in the key mapping table
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyMappingEntry {
    /// The key code
    pub key: KeyCode,
    /// Shift modifier
    pub shift: bool,
    /// Ctrl modifier
    pub ctrl: bool,
    /// Alt modifier
    pub alt: bool,
    /// Meta/Super modifier
    pub meta: bool,
}

impl KeyMappingEntry {
    /// Create a new key mapping entry from an InputEvent
    pub fn from_event(event: &InputEvent) -> Self {
        Self {
            key: event.key_code,
            shift: event.modifiers.shift,
            ctrl: event.modifiers.ctrl,
            alt: event.modifiers.alt,
            meta: event.modifiers.meta,
        }
    }
}

impl InputConfig {
    /// Create a new empty input configuration
    pub fn new() -> Self {
        InputConfig {
            device_path: None,
            key_mapping: HashMap::new(),
        }
    }

    /// Get the device path
    pub fn get_device_path(&self) -> Option<String> {
        self.device_path.clone()
    }

    /// Set the device path
    pub fn set_device_path(&mut self, path: String) {
        self.device_path = Some(path);
    }

    /// Map a key combination to a command
    pub fn map_key(&mut self, key: KeyCode, modifiers: Modifiers, command: commands::Commands) {
        let entry = KeyMappingEntry {
            key,
            shift: modifiers.shift,
            ctrl: modifiers.ctrl,
            alt: modifiers.alt,
            meta: modifiers.meta,
        };

        self.key_mapping.insert(entry, command);
    }

    /// Get the command mapped to a key combination
    pub fn get_command(&self, event: &InputEvent) -> Option<&commands::Commands> {
        let entry = KeyMappingEntry::from_event(event);
        self.key_mapping.get(&entry)
    }

    /// Best effort to guess the device path.
    pub fn guess_device_path(&mut self) {
        #[cfg(all(feature = "linux", target_os = "linux"))]
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

        #[cfg(not(all(feature = "linux", target_os = "linux")))]
        {
            warn!("Device path guessing not implemented for this platform.");
        }
    }
}

/// Input system to handle keyboard events
pub struct InputSystem {
    /// Input event receiver channel
    event_receiver: Receiver<InputEvent>,
    /// Command sender channel
    command_sender: Sender<commands::Commands>,
    /// Input handler
    handler: Option<input::InputHandler>,
    /// Configuration
    config: InputConfig,
}

impl InputSystem {
    /// Create a new input system
    pub fn new(config: InputConfig) -> Result<(Self, Receiver<commands::Commands>), InputError> {
        let (command_sender, command_receiver) = mpsc::channel();
        let (event_sender, event_receiver) = mpsc::channel();

        // Create a callback that processes inputs and sends commands
        let config_clone = config.clone();
        let command_sender_clone = command_sender.clone();
        let callback = Box::new(move |event: InputEvent| {
            if event.action == KeyAction::Press {
                if let Some(command) = config_clone.get_command(&event) {
                    if let Err(e) = command_sender_clone.send(command.clone()) {
                        error!("Failed to send command: {}", e);
                    }
                }
            }
        });

        let handler = input::InputHandler::new(&config, callback)?;

        Ok((
            Self {
                event_receiver,
                command_sender,
                handler: Some(handler),
                config,
            },
            command_receiver,
        ))
    }

    /// Start the input system
    pub fn start(&mut self) -> Result<(), InputError> {
        if self.handler.is_none() {
            // Recreate handler if needed
            let config_clone = self.config.clone();
            let command_sender_clone = self.command_sender.clone();
            let callback = Box::new(move |event: InputEvent| {
                if event.action == KeyAction::Press {
                    if let Some(command) = config_clone.get_command(&event) {
                        if let Err(e) = command_sender_clone.send(command.clone()) {
                            error!("Failed to send command: {}", e);
                        }
                    }
                }
            });

            self.handler = Some(input::InputHandler::new(&self.config, callback)?);
        }

        Ok(())
    }

    /// Stop the input system
    pub fn stop(&mut self) {
        if let Some(mut handler) = self.handler.take() {
            handler.stop();
        }
    }

    /// Poll for input events (non-blocking)
    pub fn poll_event(&self) -> Option<InputEvent> {
        match self.event_receiver.try_recv() {
            Ok(event) => Some(event),
            Err(_) => None,
        }
    }

    /// Wait for an input event (blocking)
    pub fn wait_event(&self) -> Result<InputEvent, InputError> {
        match self.event_receiver.recv() {
            Ok(event) => Ok(event),
            Err(e) => Err(InputError::EventProcessingError(e.to_string())),
        }
    }

    /// Create a default key mapping
    pub fn create_default_mapping(&mut self) {
        // Clean existing mapping
        self.config.key_mapping.clear();

        // Basic controls
        self.config.map_key(
            KeyCode::Escape,
            Modifiers::default(),
            commands::Commands::Quit,
        );
        self.config
            .map_key(KeyCode::F5, Modifiers::default(), commands::Commands::Reset);

        // Top row for loop slots (1-9, 0)
        self.config.map_key(
            KeyCode::Key1,
            Modifiers::default(),
            commands::Commands::LoadLoopFromSlot(1, 0),
        );
        self.config.map_key(
            KeyCode::Key2,
            Modifiers::default(),
            commands::Commands::LoadLoopFromSlot(2, 0),
        );
        self.config.map_key(
            KeyCode::Key3,
            Modifiers::default(),
            commands::Commands::LoadLoopFromSlot(3, 0),
        );
        self.config.map_key(
            KeyCode::Key4,
            Modifiers::default(),
            commands::Commands::LoadLoopFromSlot(4, 0),
        );
        self.config.map_key(
            KeyCode::Key5,
            Modifiers::default(),
            commands::Commands::LoadLoopFromSlot(5, 0),
        );
        self.config.map_key(
            KeyCode::Key6,
            Modifiers::default(),
            commands::Commands::LoadLoopFromSlot(6, 0),
        );
        self.config.map_key(
            KeyCode::Key7,
            Modifiers::default(),
            commands::Commands::LoadLoopFromSlot(7, 0),
        );
        self.config.map_key(
            KeyCode::Key8,
            Modifiers::default(),
            commands::Commands::LoadLoopFromSlot(8, 0),
        );
        self.config.map_key(
            KeyCode::Key9,
            Modifiers::default(),
            commands::Commands::LoadLoopFromSlot(9, 0),
        );
        self.config.map_key(
            KeyCode::Key0,
            Modifiers::default(),
            commands::Commands::LoadLoopFromSlot(0, 0),
        );

        // Same with shift to save loops
        let shift_mod = Modifiers {
            shift: true,
            ctrl: false,
            alt: false,
            meta: false,
        };
        self.config.map_key(
            KeyCode::Key1,
            shift_mod,
            commands::Commands::SaveLoopToSlot(1, 0),
        );
        self.config.map_key(
            KeyCode::Key2,
            shift_mod,
            commands::Commands::SaveLoopToSlot(2, 0),
        );
        self.config.map_key(
            KeyCode::Key3,
            shift_mod,
            commands::Commands::SaveLoopToSlot(3, 0),
        );
        self.config.map_key(
            KeyCode::Key4,
            shift_mod,
            commands::Commands::SaveLoopToSlot(4, 0),
        );
        self.config.map_key(
            KeyCode::Key5,
            shift_mod,
            commands::Commands::SaveLoopToSlot(5, 0),
        );
        self.config.map_key(
            KeyCode::Key6,
            shift_mod,
            commands::Commands::SaveLoopToSlot(6, 0),
        );
        self.config.map_key(
            KeyCode::Key7,
            shift_mod,
            commands::Commands::SaveLoopToSlot(7, 0),
        );
        self.config.map_key(
            KeyCode::Key8,
            shift_mod,
            commands::Commands::SaveLoopToSlot(8, 0),
        );
        self.config.map_key(
            KeyCode::Key9,
            shift_mod,
            commands::Commands::SaveLoopToSlot(9, 0),
        );
        self.config.map_key(
            KeyCode::Key0,
            shift_mod,
            commands::Commands::SaveLoopToSlot(0, 0),
        );

        // First row (QWERTY...)
        self.config.map_key(
            KeyCode::Q,
            Modifiers::default(),
            commands::Commands::NoteStart(0, 4, 0, 1.0),
        ); // C4
        self.config.map_key(
            KeyCode::W,
            Modifiers::default(),
            commands::Commands::NoteStart(2, 4, 0, 1.0),
        ); // D4
        self.config.map_key(
            KeyCode::E,
            Modifiers::default(),
            commands::Commands::NoteStart(4, 4, 0, 1.0),
        ); // E4
        self.config.map_key(
            KeyCode::R,
            Modifiers::default(),
            commands::Commands::NoteStart(5, 4, 0, 1.0),
        ); // F4
        self.config.map_key(
            KeyCode::T,
            Modifiers::default(),
            commands::Commands::NoteStart(7, 4, 0, 1.0),
        ); // G4
        self.config.map_key(
            KeyCode::Y,
            Modifiers::default(),
            commands::Commands::NoteStart(9, 4, 0, 1.0),
        ); // A4
        self.config.map_key(
            KeyCode::U,
            Modifiers::default(),
            commands::Commands::NoteStart(11, 4, 0, 1.0),
        ); // B4
        self.config.map_key(
            KeyCode::I,
            Modifiers::default(),
            commands::Commands::NoteStart(0, 5, 0, 1.0),
        ); // C5

        // Second row (ASDFG...)
        self.config.map_key(
            KeyCode::A,
            Modifiers::default(),
            commands::Commands::NoteStart(0, 4, 1, 1.0),
        ); // C4 on second instrument
        self.config.map_key(
            KeyCode::S,
            Modifiers::default(),
            commands::Commands::NoteStart(2, 4, 1, 1.0),
        ); // D4 on second instrument
        self.config.map_key(
            KeyCode::D,
            Modifiers::default(),
            commands::Commands::NoteStart(4, 4, 1, 1.0),
        ); // E4 on second instrument
        self.config.map_key(
            KeyCode::F,
            Modifiers::default(),
            commands::Commands::NoteStart(5, 4, 1, 1.0),
        ); // F4 on second instrument

        // Loop controls
        self.config.map_key(
            KeyCode::Space,
            Modifiers::default(),
            commands::Commands::StartRecording,
        );
        self.config.map_key(
            KeyCode::Enter,
            Modifiers::default(),
            commands::Commands::StopRecording,
        );
        self.config.map_key(
            KeyCode::P,
            Modifiers::default(),
            commands::Commands::PlayLoop,
        );
        self.config
            .map_key(KeyCode::P, shift_mod, commands::Commands::StopLoop);

        // Octave controls
        self.config.map_key(
            KeyCode::Z,
            Modifiers::default(),
            commands::Commands::OctaveDown(0),
        );
        self.config.map_key(
            KeyCode::X,
            Modifiers::default(),
            commands::Commands::OctaveUp(0),
        );
        self.config
            .map_key(KeyCode::Z, shift_mod, commands::Commands::OctaveDown(1));
        self.config
            .map_key(KeyCode::X, shift_mod, commands::Commands::OctaveUp(1));

        // Tempo controls
        self.config.map_key(
            KeyCode::Minus,
            Modifiers::default(),
            commands::Commands::TempoDown,
        );
        self.config.map_key(
            KeyCode::Equals,
            Modifiers::default(),
            commands::Commands::TempoUp,
        );

        // This is just a starting point - a real implementation would map many more keys
    }
}
