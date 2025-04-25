//! Input handling traits and platform-specific implementations

use std::sync::mpsc::Sender;

/// Common trait for all input backends
pub trait InputBackend {
    /// Start the input handling loop in a separate thread
    /// Returns a result with the Sender to send commands to the backend or an error
    fn start(config: &crate::inputs::InputConfig) -> Result<Self, InputError>
    where
        Self: Sized;

    /// Stop the input handling loop
    fn stop(&mut self);

    /// Check if the backend is running
    fn is_running(&self) -> bool;

    /// Get the sender to communicate with input backend
    fn get_sender(&self) -> Option<&Sender<InputControl>>;
}

/// Input backend control messages
pub enum InputControl {
    /// Stop the input handling loop
    Stop,
    /// Reconfigure the input handling
    Reconfigure(crate::inputs::InputConfig),
    /// Custom control message for platform-specific backends
    Custom(Box<dyn std::any::Any + Send>),
}

/// Input event type representing a key press or release
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    /// Key was pressed
    Press,
    /// Key was released
    Release,
}

/// Input event representing a key and its action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputEvent {
    /// The key code
    pub key_code: KeyCode,
    /// The action (press or release)
    pub action: KeyAction,
    /// Modifiers active during the event (Shift, Ctrl, Alt, etc.)
    pub modifiers: Modifiers,
}

/// Key modifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    /// Shift key is pressed
    pub shift: bool,
    /// Control key is pressed
    pub ctrl: bool,
    /// Alt key is pressed
    pub alt: bool,
    /// Super/Windows/Command key is pressed
    pub meta: bool,
}

/// Cross-platform key codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    // Letters
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    // Numbers
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
    Key0,

    // Function keys
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,

    // Special keys
    Escape,
    Tab,
    CapsLock,
    Shift,
    Ctrl,
    Alt,
    Meta,
    Space,
    Enter,
    Backspace,
    Delete,

    // Arrow keys
    Up,
    Down,
    Left,
    Right,

    // Numpad
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadAdd,
    NumpadSubtract,
    NumpadMultiply,
    NumpadDivide,
    NumpadEnter,

    // Other keys
    Minus,
    Equals,
    LeftBracket,
    RightBracket,
    Semicolon,
    Quote,
    Backslash,
    Comma,
    Period,
    Slash,

    // Unknown key
    Unknown,
}

/// Input handling errors
#[derive(Debug, thiserror::Error)]
pub enum InputError {
    #[error("Device not found")]
    DeviceNotFound,

    #[error("Failed to initialize input backend: {0}")]
    InitializationError(String),

    #[error("Input backend is not supported on this platform")]
    UnsupportedPlatform,

    #[error("Input event processing error: {0}")]
    EventProcessingError(String),

    #[error("Other error: {0}")]
    Other(String),
}

/// Input callback function type
pub type InputCallback = dyn Fn(InputEvent) + Send + 'static;

/// Input handling event loop
pub struct InputHandler {
    backend: Box<dyn InputBackend>,
    callback: Box<InputCallback>,
}

impl InputHandler {
    /// Create a new input handler with the given callback
    pub fn new(
        config: &crate::inputs::InputConfig,
        callback: Box<InputCallback>,
    ) -> Result<Self, InputError> {
        let backend = Self::create_backend(config)?;

        Ok(Self { backend, callback })
    }

    /// Create the appropriate backend for the current platform
    fn create_backend(
        config: &crate::inputs::InputConfig,
    ) -> Result<Box<dyn InputBackend>, InputError> {
        #[cfg(all(feature = "linux", target_os = "linux"))]
        {
            use crate::inputs::linux::LinuxInputBackend;
            return Ok(Box::new(LinuxInputBackend::start(config)?));
        }

        #[cfg(all(feature = "windows", target_os = "windows"))]
        {
            use crate::inputs::windows::WindowsInputBackend;
            return Ok(Box::new(WindowsInputBackend::start(config)?));
        }

        #[cfg(all(feature = "macos", target_os = "macos"))]
        {
            use crate::inputs::macos::MacOSInputBackend;
            return Ok(Box::new(MacOSInputBackend::start(config)?));
        }

        Err(InputError::UnsupportedPlatform)
    }

    /// Stop the input handler
    pub fn stop(&mut self) {
        self.backend.stop();
    }

    /// Check if the input handler is running
    pub fn is_running(&self) -> bool {
        self.backend.is_running()
    }
}
