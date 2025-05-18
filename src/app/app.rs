use std::default::Default;
use std::path::{Path, PathBuf};

use clap::Parser;
use log::{error, info, trace};
use serde::{Deserialize, Serialize};

use super::cli::Cli;
use super::filesystem::FSConfig;
use super::system::SystemConfig;
use crate::inputs::commands::Commands;
use crate::inputs::{InputConfig, InputError, InputSystem};
use crate::instruments::prelude::Keyboard;
use crate::prelude::Instrument;

use super::row::Row;

#[derive(Debug, Default, Deserialize, Serialize)]
/// The application configuration
pub struct AppConfig {
    pub input: InputConfig,
    pub fs: FSConfig,
    pub system: SystemConfig,
}

#[derive(Default)]
pub enum InputSystemConfig {
    External,
    #[default]
    Auto,
}

#[derive(Default)]
pub enum RunMode {
    Live(InputSystemConfig),
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
            instruments: vec![Box::new(Keyboard::<4>::new())],
            rows: [Row::default(), Row::default()],
            buffer: Vec::new(),
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

        if args.score.is_some() {
            app.run_mode = RunMode::Score;
        }

        if args.live {
            app.run_mode = RunMode::Live(if args.external_input {
                InputSystemConfig::External
            } else {
                InputSystemConfig::Auto
            });
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

    fn setup_input_system(&self) -> Result<InputSystem, InputError> {
        info!("Starting the input system");
        // Setup the input system
        let input_config = InputConfig::new();
        let (mut input_system, _cmd_receiver) = InputSystem::new(input_config)?;
        input_system.start()?;
        Ok(input_system)
    }

    pub fn on_event(&mut self, event: crate::inputs::commands::Commands) {
        match event {
            Commands::NoteStart(note, row, force) => {
                if row > 2 {
                    panic!("Row out of bounds");
                }

                let note = self.rows[row as usize].get_note(note);
                self.instruments[self.rows[row as usize].instrument].start_note(note, force);
            }
            Commands::NoteStop(note, row) => {
                if row > 2 {
                    panic!("Row out of bounds");
                }

                let note = self.rows[row as usize].get_note(note);
                self.instruments[self.rows[row as usize].instrument].stop_note(note);
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
            RunMode::Live(config) => {
                info!("Starting live mode");
                match config {
                    InputSystemConfig::Auto => {
                        let input_system = match self.setup_input_system() {
                            Ok(input_system) => input_system,
                            Err(err) => {
                                error!("Failed to setup input system: {}", err);
                                return;
                            }
                        };

                        loop {
                            // Handle input events
                            if let Some(event) = input_system.poll_event() {
                                // Convert input events to Commands and tick the app
                                println!("Received event: {:?}", event);
                            }
                        }
                    }
                    InputSystemConfig::External => {
                        // Update to use here instead ?
                    }
                }
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
            RunMode::Live(_) => {
                trace!("Live ticking");
                self.live_tick();
            }
        }
    }
}
