//! The `app` module contains the main application data structures and functions.
//! It provides CLI utilities for managing the application as well as filesystem
//! utilities for managing files and directories.

use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

use clap::Parser;
use cpal::SampleRate;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use log::info;

use super::commands::SystemCommand;
use super::prelude::*;
use super::state::AppState;
use super::{AppMode, config::AppConfig};
use crate::app::error::AppError;
use crate::audio::{AudioError, AudioHandle, BackendEvent};
use crate::prelude::Instrument;

/// Application metaobject, contains the application's configuration,
/// Available instruments, paths to save/load files to/from, ...
pub struct App {
    pub config: AppConfig,
    pub state: Arc<Mutex<AppState>>,
    pub instruments: Vec<Box<dyn Instrument + Send + Sync>>, // All the instruments loaded in the app

    pub handle: Option<AudioHandle>,
    pub command_tx: Option<Sender<Command>>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            config: AppConfig::default(),
            state: Arc::new(Mutex::new(AppState::default())),
            instruments: Vec::new(),

            handle: None,
            command_tx: None,
        }
    }
}

impl App {
    /// Creates a new App with default configuration.
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
                Err(e) => println!("Unable to dump config: {e}"),
            }
            std::process::exit(0);
        }

        app
    }

    /// Starts the engine. Spawns the command, render and cpal threads
    /// and holds the handles. Returns the receiver for backend events.
    pub fn start(&mut self) -> Result<Receiver<BackendEvent>, AudioError> {
        let (command_tx, command_rx): (Sender<Command>, Receiver<Command>) = channel();
        let (event_tx, event_rx): (Sender<BackendEvent>, Receiver<BackendEvent>) = channel();

        info!("Starting audio engine");
        self.config
            .audio
            .validate()
            .map_err(AudioError::ConfigError)?;

        let config = self.config.audio.clone();
        let shared_state = Arc::new(crate::audio::SharedAudioState::new());

        use crossbeam::queue::ArrayQueue;
        let audio_queue = Arc::new(ArrayQueue::<f32>::new(config.audio_ring_buffer_size));
        let audio_queue_producer = audio_queue.clone();
        let audio_queue_consumer = audio_queue;

        let (message_tx, message_rx) = crossbeam::channel::bounded(config.message_ring_buffer_size);

        let instruments = std::mem::take(&mut self.instruments);

        let host = cpal::default_host();
        let device = host.default_output_device().ok_or(AudioError::NoDevice)?;

        let mut supported_configs_range = device
            .supported_output_configs()
            .map_err(|e| AudioError::StreamError(e.to_string()))?;

        let supported_config = supported_configs_range
            .find(|c| c.channels() == crate::core::audio::CHANNELS as u16)
            .or_else(|| {
                device
                    .supported_output_configs()
                    .ok()
                    .and_then(|mut r| r.next())
            })
            .ok_or(AudioError::StreamError("No supported config".to_string()))?
            .with_sample_rate(SampleRate(44100));

        let mut cpal_config = supported_config.config();
        cpal_config.buffer_size = cpal::BufferSize::Fixed(config.cpal_buffer_size as u32);

        let sample_rate = cpal_config.sample_rate.0;
        self.config.system.sample_rate = sample_rate;
        shared_state
            .sample_rate
            .store(sample_rate, Ordering::Relaxed);

        info!(
            "Audio config: sample_rate={sample_rate}, buffer_size={}, ring_buffer={}",
            config.cpal_buffer_size, config.audio_ring_buffer_size
        );

        let render_thread = crate::audio::spawn_audio_render_thread(
            shared_state.clone(),
            instruments,
            message_rx,
            audio_queue_producer,
            config.clone(),
            event_tx.clone(),
        );

        let app_state = self.state.clone();
        let command_thread = crate::audio::spawn_command_thread(
            app_state,
            shared_state.clone(),
            command_rx,
            event_tx.clone(),
            message_tx,
        );

        let callback =
            crate::audio::create_cpal_callback(audio_queue_consumer, shared_state.clone());

        let stream = device
            .build_output_stream(
                &cpal_config,
                callback,
                move |err| {
                    log::error!("Audio stream error: {}", err);
                },
                None,
            )
            .map_err(|e| AudioError::StreamError(e.to_string()))?;

        stream
            .play()
            .map_err(|e| AudioError::StreamError(e.to_string()))?;

        let _ = event_tx.send(BackendEvent::AudioStarted { sample_rate });

        self.handle = Some(AudioHandle::new(
            command_thread,
            render_thread,
            stream,
            shared_state,
        ));
        self.command_tx = Some(command_tx);

        Ok(event_rx)
    }

    /// Send a command to the audio engine.
    pub fn send(&self, command: Command) -> Result<(), AppError> {
        self.command_tx
            .as_ref()
            .ok_or(AppError::NotStarted)?
            .send(command)
            .map_err(|_| AppError::ChannelClosed)
    }

    /// Stops the engine, sends shutdown, joins the threads.
    pub fn stop(&mut self) -> Result<(), AppError> {
        if let Some(ref tx) = self.command_tx {
            let _ = tx.send(Command::Audio(AudioCommand::Shutdown));
        }
        if let Some(handle) = self.handle.take() {
            handle
                .shutdown()
                .map_err(|e| AppError::AudioError(e.to_string()))?;
        }
        self.command_tx = None;
        Ok(())
    }

    /// Tries to load the application configuration from a file.
    pub fn from_file(path: &Path) -> Result<App, AppError> {
        Ok(App {
            config: AppConfig::from_file(path)?,
            state: Arc::new(Mutex::new(AppState {
                mode: AppMode::Setup,
            })),
            instruments: Vec::new(),
            handle: None,
            command_tx: None,
        })
    }

    pub fn handle_system_command(&mut self, event: SystemCommand) {
        match event {
            SystemCommand::Reset => {
                log::error!("Not implemented System::Reset");
            }
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        // Best effort safety net, call `stop` explicitly!
        // ignore error
        let _ = self.stop();
    }
}
