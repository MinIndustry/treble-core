#[allow(clippy::module_inception)]
mod app;
pub mod audio_graph;
pub mod commands;
pub(crate) mod graph_handler;

/// The filesystem module is used to interact with the filesystem.
/// Its purpose is to help organize the filesystem and provide a way to interact with it.
mod filesystem;

mod config;
mod error;
/// The System module is used to interact with the system.
/// Its purpose is to help organize the system and provide a way to interact with it.
mod system;

pub mod state;

#[derive(Default)]
pub enum AppMode {
    #[default]
    Setup, // Setup mode, the app has not started yet
    Input,   // Waiting for user input
    Running, // Simply running, use inputs as commands/notes
}

// Export essential types directly from the app module
pub mod prelude {
    pub use super::AppMode;
    pub use super::app::App;
    pub use super::audio_graph::{AudioGraph, InstrumentSlot};
    pub use super::commands::{AppCommand, AudioCommand, Command};
    pub use super::filesystem::FSConfig;
    pub use super::state::AppState;
    pub use super::system::SystemConfig;
}
