use std::collections::HashMap;
use std::default::Default;
use std::path::{Path, PathBuf};

use clap::Parser;
use log::{error, info};
use serde::{Deserialize, Serialize};

use super::cli::Cli;
use super::filesystem::FSConfig;
use super::system::SystemConfig;
use crate::core::keys;
use crate::inputs::{InputConfig, InputSystem};
use crate::note;

#[derive(Debug, Default, Deserialize, Serialize)]
/// The application configuration
pub struct AppConfig {
    pub input: InputConfig,
    pub fs: FSConfig,
    pub system: SystemConfig,
}

#[derive(Default)]
pub enum RunMode {
    Live,
    Score,
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

/// Application meta-object, contains the application's configuration,
/// Available instruments, paths to save/load files to/from, ...
pub struct App {
    pub config: AppConfig,
    pub run_mode: RunMode,
    pub mode: AppMode,
}

impl Default for App {
    fn default() -> Self {
        let root_path = match crate::app::FSConfig::app_root_dir() {
            Ok(path) => path,
            Err(e) => {
                error!("Unable to build app root dir: {}", e);
                PathBuf::from("./")
            }
        };

        let config_file = root_path.join("config.toml");

        let config = if config_file.exists() {
            match toml::from_str(&std::fs::read_to_string(config_file).unwrap()) {
                Ok(config) => config,
                Err(e) => {
                    error!("Unable to parse config file: {}", e);
                    AppConfig::default()
                }
            }
        } else {
            AppConfig::default()
        };

        Self {
            config,
            run_mode: RunMode::Unknown,
            mode: AppMode::Input,
        }
    }
}

impl App {
    /// Tries to load the application configuration from a default path.
    /// If the configuration file does not exist, it will use the default configuration.
    pub fn new() -> App {
        App::default()
    }

    /// Initializes the application settings from the command line arguments.
    /// This function is susceptible to terminate the process (e.g. when the command
    /// line arguments ask for the application version or a dump of the config).
    pub fn init() -> App {
        let args = Cli::parse();
        let mut app = if let Some(path) = args.config {
            App::from_file(&path)
                .map_err(|e| {
                    println!("Unable to load config: {}", e);
                    std::process::exit(1);
                })
                .unwrap()
        } else {
            App::default()
        };

        if args.dump_config {
            match toml::to_string(&app.config) {
                Ok(s) => println!("{}", s),
                Err(e) => println!("Unable to dump config: {}", e.to_string()),
            }
            std::process::exit(0);
        }

        if args.score.is_none() && !args.live {
            println!("No score or live mode specified");
            std::process::exit(1);
        }

        if args.score.is_some() {
            app.run_mode = RunMode::Score;
        }

        if args.live {
            app.run_mode = RunMode::Live;
        }

        app
    }

    /// Tries to load the application configuration from a file.
    pub fn from_file(path: &Path) -> Result<App, String> {
        info!(
            "Loading configuration from file: {}",
            path.to_str().unwrap_or("unknown")
        );
        let config: AppConfig =
            toml::from_str(&std::fs::read_to_string(path).map_err(|e| e.to_string())?)
                .map_err(|e| format!("Unable to load config: {}", e))?;

        Ok(App {
            config,
            run_mode: RunMode::Unknown,
            mode: AppMode::Setup,
        })
    }

    pub fn live_mode(&mut self) {
        info!("Starting the input system");
        // Setup the input system
        let input_config = InputConfig::new();
        let (mut input_system, cmd_receiver) = match InputSystem::new(input_config) {
            Ok(system) => system,
            Err(e) => panic!("Failed to create input system: {}", e),
        };

        match input_system.start() {
            Ok(_) => {
                info!("Input system started successfully");
            }
            Err(e) => {
                error!("Failed to start input system: {}", e);
            }
        }
        loop {
            // Handle input events
            if let Some(event) = input_system.poll_event() {
                // Process input events
                println!("Received event: {:?}", event);
            }
        }
    }

    /// Runs the application
    pub fn run(&mut self) {
        info!("Running application");
        match self.run_mode {
            RunMode::Unknown => {
                panic!("Unknown run mode");
            }
            RunMode::Score => {
                info!("Running score mode");
            }
            RunMode::Live => {
                self.live_mode();
            }
        }
    }

    pub fn get_key_mapping(&self) -> HashMap<u16, keys::Key> {
        // TODO: Load an actual key mapping
        HashMap::from([
            (16, note!(keys::KeyCode::NoteC)),
            (17, note!(keys::KeyCode::NoteCS)),
            (18, note!(keys::KeyCode::NoteD)),
            (19, note!(keys::KeyCode::NoteDS)),
            (20, note!(keys::KeyCode::NoteE)),
            (21, note!(keys::KeyCode::NoteF)),
            (22, note!(keys::KeyCode::NoteFS)),
            (23, note!(keys::KeyCode::NoteG)),
            (24, note!(keys::KeyCode::NoteGS)),
            (25, note!(keys::KeyCode::NoteA)),
            (26, note!(keys::KeyCode::NoteAS)),
            (27, note!(keys::KeyCode::NoteB)),
        ])
    }
}
