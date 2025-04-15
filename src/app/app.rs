use std::collections::HashMap;
use std::default::Default;
use std::path::PathBuf;

use clap::Parser;
use log::{error, info};
use serde::{Deserialize, Serialize};

use super::cli::Cli;
use super::filesystem::FSConfig;
use super::system::SystemConfig;
use crate::core::keys;
use crate::inputs::InputConfig;
use crate::note;

#[derive(Debug, Default, Deserialize, Serialize)]
/// The application configuration
pub struct AppConfig {
    pub input: InputConfig,
    pub fs: FSConfig,
    pub system: SystemConfig,
}

#[derive(Deserialize, Debug)]
/// Application meta-object, contains the application's configuration,
/// Available instruments, paths to save/load files to/from, ...
pub struct App {
    pub config: AppConfig,
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

        if config_file.exists() {
            match toml::from_str(&std::fs::read_to_string(config_file).unwrap()) {
                Ok(config) => App { config },
                Err(e) => {
                    error!("Unable to parse config file: {}", e);
                    Self {
                        config: AppConfig::default(),
                    }
                }
            }
        } else {
            Self {
                config: AppConfig::default(),
            }
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
        let app = if let Some(path) = args.config {
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

        app
    }

    /// Tries to load the application configuration from a file.
    pub fn from_file(path: &String) -> Result<App, String> {
        info!("Loading configuration from file: {}", path);
        toml::from_str(&std::fs::read_to_string(path).map_err(|e| e.to_string())?)
            .map_err(|e| format!("Unable to load config: {}", e))
    }

    pub fn run(&self) {
        info!("Running application");
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
