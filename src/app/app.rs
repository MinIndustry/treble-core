use clap::Parser;

use log::info;
use serde::Deserialize;
use std::collections::HashMap;
use std::default::Default;

use crate::core::keys;
use crate::note;

use super::cli::Cli;
use super::config::AppConfig;

#[derive(Deserialize, Default, Debug)]
/// Application meta-object, contains the application's configuration,
/// Available instruments, paths to save/load files to/from, ...
pub struct App {
    pub config: AppConfig,
}

impl App {
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

    pub fn from_file(path: &String) -> App {
        info!("Loading configuration from file: {}", path);

        App::new()
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
