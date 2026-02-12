#[allow(clippy::module_inception)]
mod app;
mod cli;
pub mod commands;
mod row;

/// The filesystem module is used to interact with the filesystem.
/// Its purpose is to help organize the filesystem and provide a way to interact with it.
mod filesystem;

mod config;
mod error;
/// The System module is used to interact with the system.
/// Its purpose is to help organize the system and provide a way to interact with it.
mod system;

#[derive(Default)]
pub enum RunMode {
    Live,  // App is ready to play live, has loaded instruments
    Score, // App is ready to play a score
    Graph, // App is ready to play a graph or multiple graphs
    #[default]
    Unknown,
}

#[derive(Default)]
pub enum AppMode {
    #[default]
    Setup, // Setup mode, the app has not started yet
    Input,   // Waiting for user input
    Running, // Simply running, use inputs as commands/notes
}

// Export essential types directly from the app module
pub mod prelude {
    pub use super::app::App;
    pub use super::cli::Cli;
    pub use super::commands::Command;
    pub use super::filesystem::FSConfig;
    pub use super::system::SystemConfig;
    pub use super::{AppMode, RunMode};
}
