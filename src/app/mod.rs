mod app;
mod cli;

/// The filesystem module is used to interact with the filesystem.
/// Its purpose is to help organize the filesystem and provide a way to interact with it.
mod filesystem;

/// The System module is used to interact with the system.
/// Its purpose is to help organize the system and provide a way to interact with it.
mod system;

// Export essential types directly from the app module
pub use app::{App, AppMode, RunMode};
pub use cli::Cli;
pub use filesystem::FSConfig;
pub use system::SystemConfig;
