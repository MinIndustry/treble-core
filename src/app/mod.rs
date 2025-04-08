mod app;
mod cli;

/// The filesystem module is used to interact with the filesystem.
/// Its purpose is to help organize the filesystem and provide a way to interact with it.
mod filesystem;

/// The System module is used to interact with the system.
/// Its purpose is to help organize the system and provide a way to interact with it.
mod system;

pub mod prelude {
    pub use super::app::App;
    pub use super::cli::Cli;
    pub use super::filesystem::FSConfig;
    pub use super::system::SystemConfig;
}
