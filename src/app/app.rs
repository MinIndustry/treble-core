//! The `app` module contains the main application data structures and functions.
//! It provides CLI utilities for managing the application as well as filesystem
//! utilities for managing files and directories.

use std::default::Default;
use std::path::Path;

use clap::Parser;
use log::{info, trace};

use super::commands::{LiveCommand, SystemCommand};
use super::prelude::*;
use crate::app::error::AppError;
use crate::instruments::prelude::KeyboardBuilder;
use crate::prelude::Instrument;

use super::{AppMode, RunMode, config::AppConfig, row::Row};

/// Application metaobject, contains the application's configuration,
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
        Self {
            config: AppConfig::default(),
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
    pub fn from_file(path: &Path) -> Result<App, AppError> {
        Ok(App {
            config: AppConfig::from_file(path)?,
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

    pub fn handle_system_command(&mut self, event: SystemCommand) {
        match event {
            SystemCommand::Reset => {
                log::error!("Not implemented System::Reset");
            }
        }
    }

    pub fn handle_live_command(&mut self, event: LiveCommand) {
        match event {
            LiveCommand::OctaveUp(row) => {
                self.rows[row as usize].octave += 1;
            }
            LiveCommand::OctaveDown(row) => {
                self.rows[row as usize].octave -= 1;
            }
            _ => {}
        }
    }

    pub fn on_event(&mut self, event: AppCommand) {
        match event {
            AppCommand::System(system_command) => self.handle_system_command(system_command),
            AppCommand::Live(live_command) => self.handle_live_command(live_command),
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
            RunMode::Graph => {
                info!("Starting graph mode");
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
            RunMode::Graph => {
                info!("Running graph mode");
            }
        }
    }
}
