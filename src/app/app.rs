//! The `app` module contains the main application data structures and functions.
//! It provides CLI utilities for managing the application as well as filesystem
//! utilities for managing files and directories.

use std::default::Default;
use std::path::{Path, PathBuf};

use clap::Parser;
use log::{error, info, trace};
use serde::{Deserialize, Serialize};

use super::prelude::*;

use crate::instruments::prelude::KeyboardBuilder;
use crate::prelude::Instrument;

use super::row::Row;

#[derive(Debug, Default, Deserialize, Serialize)]
/// The application configuration
pub struct AppConfig {
    pub fs: FSConfig,
    pub system: SystemConfig,

    #[serde(default)]
    pub audio: crate::audio::AudioConfig,

    #[serde(default)]
    pub logging: crate::audio::LogConfig,
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
    pub rows: [Row; 2], // Mapping from keyboard rows to currently selected instruments
    pub instruments: Vec<Box<dyn Instrument + Send + Sync>>, // All the instruments loaded in the app
    pub buffer: Vec<f32>,                                    // Buffer for audio data
}

impl Default for App {
    fn default() -> Self {
        let root_path = match FSConfig::app_root_dir() {
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
            instruments: vec![Box::new(KeyboardBuilder::new().build())],
            rows: [Row::default(), Row::default()],
            buffer: Vec::new(),
        }
    }
}

impl App {
    /// Tries to load the application configuration from a default path.
    /// If the configuration file does not exist, it will use the default configuration.
    pub fn new() -> App {
        let mut app = App::default();

        app.rows[0].octave = 3;
        app
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
                Err(e) => println!("Unable to dump config: {e}"),
            }
            std::process::exit(0);
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
            rows: [Row::default(), Row::default()],
            instruments: Vec::new(),
            buffer: Vec::new(),
        })
    }

    /// Sets the application mode.
    pub fn set_mode(&mut self, mode: RunMode) {
        self.run_mode = mode;
    }

    pub fn on_event(&mut self, event: Commands) {
        match event {
            Commands::NoteStart(note, row, force) => {
                if row > 2 {
                    panic!("Row out of bounds");
                }

                let note = self.rows[row as usize].get_note(note);
                if self.rows[row as usize].instrument >= self.instruments.len() {
                    log::warn!("Instrument out of bounds");
                } else {
                    self.instruments[self.rows[row as usize].instrument].start_note(note, force);
                }
            }
            Commands::NoteStop(note, row) => {
                if row > 2 {
                    panic!("Row out of bounds");
                }

                let note = self.rows[row as usize].get_note(note);
                if self.rows[row as usize].instrument >= self.instruments.len() {
                    log::warn!("Instrument out of bounds");
                } else {
                    self.instruments[self.rows[row as usize].instrument].stop_note(note);
                }
            }
            Commands::OctaveDown(row) => {
                if row > 2 {
                    panic!("Row out of bounds");
                }

                self.rows[row as usize].octave -= 1;
            }
            Commands::OctaveUp(row) => {
                if row > 2 {
                    panic!("Row out of bounds");
                }

                self.rows[row as usize].octave += 1;
            }
            _ => {}
        }
    }

    pub fn live_tick(&mut self) -> f32 {
        self.instruments.iter_mut().for_each(|r| r.tick());

        self.instruments
            .iter_mut()
            .map(|row| row.get_output())
            .sum::<f32>()
    }

    /// Runs the application in standalone mode
    pub fn run(&mut self) {
        info!("Running application");
        match &self.run_mode {
            RunMode::Unknown => {
                panic!("Unknown run mode");
            }
            RunMode::Score => {
                info!("Running score mode");
            }
            RunMode::Live => {
                info!("Starting live mode");
            }
        }
    }

    /// Ticks the application, processes the events.
    pub fn tick(&mut self) {
        match self.run_mode {
            RunMode::Unknown => {
                panic!("Unknown run mode");
            }
            RunMode::Score => {
                info!("Running score mode");
            }
            RunMode::Live => {
                trace!("Live ticking");
                self.live_tick();
            }
        }
    }
}
