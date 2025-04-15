//! Linux-specific input handling using evdev

use std::sync::{mpsc::{channel, Sender, Receiver}, Arc, Mutex};
use std::thread;
use evdev::{Device, InputEvent as EvdevEvent, Key, EventType};
use log::{debug, error, warn, info};

use super::input::{InputBackend, InputError, InputControl, InputEvent, KeyAction, KeyCode, Modifiers};

pub struct LinuxInputBackend {
    device: Option<Arc<Mutex<Device>>>,
    running: Arc<Mutex<bool>>,
    thread_handle: Option<thread::JoinHandle<()>>,
    sender: Option<Sender<InputControl>>,
    callback_sender: Option<Sender<InputEvent>>,
}

impl InputBackend for LinuxInputBackend {
    fn start(config: &crate::inputs::InputConfig) -> Result<Self, InputError> {
        let device = match find_device(config) {
            Some(device) => device,
            None => return Err(InputError::DeviceNotFound),
        };
        
        let device = Arc::new(Mutex::new(device));
        let running = Arc::new(Mutex::new(true));
        
        let (control_sender, control_receiver) = channel();
        let (callback_sender, callback_receiver) = channel();
        
        let device_clone = Arc::clone(&device);
        let running_clone = Arc::clone(&running);
        
        let thread_handle = thread::spawn(move || {
            run_input_loop(device_clone, running_clone, control_receiver, callback_receiver);
        });
        
        Ok(Self {
            device: Some(device),
            running,
            thread_handle: Some(thread_handle),
            sender: Some(control_sender),
            callback_sender: Some(callback_sender),
        })
    }
    
    fn stop(&mut self) {
        if let Ok(mut running) = self.running.lock() {
            *running = false;
        }
        
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
    
    fn is_running(&self) -> bool {
        match self.running.lock() {
            Ok(running) => *running,
            Err(_) => false,
        }
    }
    
    fn get_sender(&self) -> Option<&Sender<InputControl>> {
        self.sender.as_ref()
    }
}

impl Drop for LinuxInputBackend {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Find an appropriate input device based on configuration
fn find_device(config: &crate::inputs::InputConfig) -> Option<Device> {
    // If a device path is specified in the config, try to open it
    if let Some(path) = config.get_device_path() {
        match Device::open(&path) {
            Ok(device) => {
                info!("Opened device at {}: {}", path, device.name().unwrap_or("Unknown device"));
                return Some(device);
            },
            Err(e) => {
                warn!("Failed to open device at {}: {}", path, e);
            }
        }
    }
    
    // Otherwise, try to find a suitable keyboard device
    super::keyboard::find_keyboard()
}

/// Convert an evdev key to our cross-platform KeyCode
fn convert_key(key: Key) -> KeyCode {
    match key {
        // Letters
        Key::KEY_A => KeyCode::A,
        Key::KEY_B => KeyCode::B,
        Key::KEY_C => KeyCode::C,
        Key::KEY_D => KeyCode::D,
        Key::KEY_E => KeyCode::E,
        Key::KEY_F => KeyCode::F,
        Key::KEY_G => KeyCode::G,
        Key::KEY_H => KeyCode::H,
        Key::KEY_I => KeyCode::I,
        Key::KEY_J => KeyCode::J,
        Key::KEY_K => KeyCode::K,
        Key::KEY_L => KeyCode::L,
        Key::KEY_M => KeyCode::M,
        Key::KEY_N => KeyCode::N,
        Key::KEY_O => KeyCode::O,
        Key::KEY_P => KeyCode::P,
        Key::KEY_Q => KeyCode::Q,
        Key::KEY_R => KeyCode::R,
        Key::KEY_S => KeyCode::S,
        Key::KEY_T => KeyCode::T,
        Key::KEY_U => KeyCode::U,
        Key::KEY_V => KeyCode::V,
        Key::KEY_W => KeyCode::W,
        Key::KEY_X => KeyCode::X,
        Key::KEY_Y => KeyCode::Y,
        Key::KEY_Z => KeyCode::Z,
        
        // Numbers
        Key::KEY_1 => KeyCode::Key1,
        Key::KEY_2 => KeyCode::Key2,
        Key::KEY_3 => KeyCode::Key3,
        Key::KEY_4 => KeyCode::Key4,
        Key::KEY_5 => KeyCode::Key5,
        Key::KEY_6 => KeyCode::Key6,
        Key::KEY_7 => KeyCode::Key7,
        Key::KEY_8 => KeyCode::Key8,
        Key::KEY_9 => KeyCode::Key9,
        Key::KEY_0 => KeyCode::Key0,
        
        // Function keys
        Key::KEY_F1 => KeyCode::F1,
        Key::KEY_F2 => KeyCode::F2,
        Key::KEY_F3 => KeyCode::F3,
        Key::KEY_F4 => KeyCode::F4,
        Key::KEY_F5 => KeyCode::F5,
        Key::KEY_F6 => KeyCode::F6,
        Key::KEY_F7 => KeyCode::F7,
        Key::KEY_F8 => KeyCode::F8,
        Key::KEY_F9 => KeyCode::F9,
        Key::KEY_F10 => KeyCode::F10,
        Key::KEY_F11 => KeyCode::F11,
        Key::KEY_F12 => KeyCode::F12,
        
        // Special keys
        Key::KEY_ESC => KeyCode::Escape,
        Key::KEY_TAB => KeyCode::Tab,
        Key::KEY_CAPSLOCK => KeyCode::CapsLock,
        Key::KEY_LEFTSHIFT | Key::KEY_RIGHTSHIFT => KeyCode::Shift,
        Key::KEY_LEFTCTRL | Key::KEY_RIGHTCTRL => KeyCode::Ctrl,
        Key::KEY_LEFTALT | Key::KEY_RIGHTALT => KeyCode::Alt,
        Key::KEY_LEFTMETA | Key::KEY_RIGHTMETA => KeyCode::Meta,
        Key::KEY_SPACE => KeyCode::Space,
        Key::KEY_ENTER => KeyCode::Enter,
        Key::KEY_BACKSPACE => KeyCode::Backspace,
        Key::KEY_DELETE => KeyCode::Delete,
        
        // Arrow keys
        Key::KEY_UP => KeyCode::Up,
        Key::KEY_DOWN => KeyCode::Down,
        Key::KEY_LEFT => KeyCode::Left,
        Key::KEY_RIGHT => KeyCode::Right,
        
        // Numpad
        Key::KEY_KP0 => KeyCode::Numpad0,
        Key::KEY_KP1 => KeyCode::Numpad1,
        Key::KEY_KP2 => KeyCode::Numpad2,
        Key::KEY_KP3 => KeyCode::Numpad3,
        Key::KEY_KP4 => KeyCode::Numpad4,
        Key::KEY_KP5 => KeyCode::Numpad5,
        Key::KEY_KP6 => KeyCode::Numpad6,
        Key::KEY_KP7 => KeyCode::Numpad7,
        Key::KEY_KP8 => KeyCode::Numpad8,
        Key::KEY_KP9 => KeyCode::Numpad9,
        Key::KEY_KPPLUS => KeyCode::NumpadAdd,
        Key::KEY_KPMINUS => KeyCode::NumpadSubtract,
        Key::KEY_KPASTERISK => KeyCode::NumpadMultiply,
        Key::KEY_KPSLASH => KeyCode::NumpadDivide,
        Key::KEY_KPENTER => KeyCode::NumpadEnter,
        
        // Other keys
        Key::KEY_MINUS => KeyCode::Minus,
        Key::KEY_EQUAL => KeyCode::Equals,
        Key::KEY_LEFTBRACE => KeyCode::LeftBracket,
        Key::KEY_RIGHTBRACE => KeyCode::RightBracket,
        Key::KEY_SEMICOLON => KeyCode::Semicolon,
        Key::KEY_APOSTROPHE => KeyCode::Quote,
        Key::KEY_BACKSLASH => KeyCode::Backslash,
        Key::KEY_COMMA => KeyCode::Comma,
        Key::KEY_DOT => KeyCode::Period,
        Key::KEY_SLASH => KeyCode::Slash,
        
        // Unknown keys
        _ => KeyCode::Unknown,
    }
}

/// Convert evdev event value to KeyAction
fn convert_key_action(value: i32) -> KeyAction {
    match value {
        0 => KeyAction::Release,
        1 => KeyAction::Press,
        _ => KeyAction::Release, // For repeat events, we'll treat as release
    }
}

/// Check if a key is a modifier key
fn is_modifier_key(key: Key) -> bool {
    matches!(key, 
        Key::KEY_LEFTSHIFT | 
        Key::KEY_RIGHTSHIFT | 
        Key::KEY_LEFTCTRL | 
        Key::KEY_RIGHTCTRL | 
        Key::KEY_LEFTALT | 
        Key::KEY_RIGHTALT | 
        Key::KEY_LEFTMETA | 
        Key::KEY_RIGHTMETA
    )
}

/// Update modifiers based on key and action
fn update_modifiers(modifiers: &mut Modifiers, key: Key, action: KeyAction) {
    let pressed = action == KeyAction::Press;
    match key {
        Key::KEY_LEFTSHIFT | Key::KEY_RIGHTSHIFT => modifiers.shift = pressed,
        Key::KEY_LEFTCTRL | Key::KEY_RIGHTCTRL => modifiers.ctrl = pressed,
        Key::KEY_LEFTALT | Key::KEY_RIGHTALT => modifiers.alt = pressed,
        Key::KEY_LEFTMETA | Key::KEY_RIGHTMETA => modifiers.meta = pressed,
        _ => {}
    }
}

/// Main input processing loop
fn run_input_loop(
    device: Arc<Mutex<Device>>,
    running: Arc<Mutex<bool>>,
    control_receiver: Receiver<InputControl>,
    callback_sender: Sender<InputEvent>,
) {
    let mut modifiers = Modifiers::default();
    
    'main: while let Ok(is_running) = running.lock() {
        if !*is_running {
            break 'main;
        }
        
        // Check for control messages
        if let Ok(control) = control_receiver.try_recv() {
            match control {
                InputControl::Stop => break 'main,
                InputControl::Reconfigure(_) => {
                    // Handle reconfiguration if needed
                }
                InputControl::Custom(_) => {
                    // Handle custom control messages
                }
            }
        }
        
        // Process input events
        let events = match device.lock() {
            Ok(mut device) => match device.fetch_events() {
                Ok(events) => events.collect::<Vec<_>>(),
                Err(e) => {
                    if e.kind() != std::io::ErrorKind::WouldBlock {
                        error!("Error fetching events: {}", e);
                    }
                    vec![]
                }
            },
            Err(e) => {
                error!("Error locking device: {}", e);
                vec![]
            }
        };
        
        for event in events {
            if event.event_type() == EventType::KEY {
                if let Some(key) = event.input_event_code() {
                    if let Some(key) = key.code() {
                        let key_code = convert_key(key);
                        let action = convert_key_action(event.value());
                        
                        // Update modifiers
                        if is_modifier_key(key) {
                            update_modifiers(&mut modifiers, key, action);
                        }
                        
                        // Send the event to the callback
                        let input_event = InputEvent {
                            key_code,
                            action,
                            modifiers,
                        };
                        
                        if let Err(e) = callback_sender.send(input_event) {
                            error!("Error sending input event: {}", e);
                            break 'main;
                        }
                    }
                }
            }
        }
        
        // Don't burn CPU
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    
    debug!("Input loop terminated");
}