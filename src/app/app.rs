//! Application meta-object: configuration, audio graph, and runtime state.

use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

use clap::Parser;
use cpal::SampleRate;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use log::info;

use super::commands::{AppCommand, AudioCommand, SystemCommand};
use super::graph_handler::{GraphData, handle_graph_command};
use super::prelude::*;
use super::state::AppState;
use super::{AppMode, config::AppConfig};
use crate::app::audio_graph::AudioGraph;
use crate::app::error::AppError;
use crate::audio::EventSender;
use crate::audio::{
    AudioError, AudioHandle, AudioMessage, BackendEvent, EventFilter, GraphAudioMessage,
    InstrumentAudioMessage, StatusEvent,
};
use crate::core::utils::Note;
use crate::instruments::Instrument;

/// Application meta-object.
///
/// Owns all instruments via [`AudioGraph`] and sends [`AudioMessage`]s
/// directly to the render thread after [`start()`](Self::start).
///
/// The frontend-facing API uses [`Command`] / [`AudioCommand`] with
/// `instrument_idx`; App translates them to source indices internally
/// so the render thread remains decoupled from instrument ordering.
pub struct App {
    pub config: AppConfig,
    pub state: Arc<Mutex<AppState>>,

    /// All instrument slots. Populated before `start()`, compiled on start.
    pub audio_graph: AudioGraph,

    /// State for the visual graph editor. Protected by Mutex for `send(&self)`.
    graph_system: Mutex<GraphData>,

    pub handle: Option<AudioHandle>,
    /// Direct channel to the render thread (no intermediate command thread).
    message_tx: Option<crossbeam::channel::Sender<AudioMessage>>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            config: AppConfig::default(),
            state: Arc::new(Mutex::new(AppState::default())),
            audio_graph: AudioGraph::new(),
            graph_system: Mutex::new(GraphData::default()),
            handle: None,
            message_tx: None,
        }
    }
}

impl App {
    pub fn new() -> App {
        App::default()
    }

    /// Initializes the application from CLI arguments.
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

    /// Add an instrument and return its slot index (for use with `note_on`/`note_off`).
    ///
    /// Call before [`start()`](Self::start). To add instruments at runtime,
    /// call [`recompile()`](Self::recompile) afterwards.
    pub fn add_instrument(&mut self, instrument: Box<dyn Instrument>) -> usize {
        self.audio_graph.add_instrument(instrument)
    }

    /// Recompile the audio graph and hot-swap it into the running render thread.
    pub fn recompile(&mut self) -> Result<(), AppError> {
        let system = self
            .audio_graph
            .compile()
            .map_err(|e| AppError::AudioError(format!("{:?}", e)))?;
        self.send_message(AudioMessage::Graph(GraphAudioMessage::Swap(system)))
    }

    /// Start the audio engine.
    ///
    /// Compiles the instrument graph, spawns the render thread, and returns
    /// a receiver for backend events. Pass an [`EventFilter`] to control which
    /// event categories are forwarded; use [`EventFilter::all()`] to receive everything.
    pub fn start(&mut self, filter: EventFilter) -> Result<Receiver<BackendEvent>, AudioError> {
        let (raw_tx, event_rx): (Sender<BackendEvent>, Receiver<BackendEvent>) = channel();
        let event_tx = EventSender::new(raw_tx, filter);

        info!("Starting audio engine");
        self.config
            .audio
            .validate()
            .map_err(AudioError::ConfigError)?;

        let config = self.config.audio.clone();
        let shared_state = Arc::new(crate::audio::SharedAudioState::new());

        use crossbeam::queue::ArrayQueue;
        let audio_queue = Arc::new(ArrayQueue::<f32>::new(config.audio_ring_buffer_size));

        let (message_tx, message_rx) = crossbeam::channel::bounded(config.message_ring_buffer_size);

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

        let compiled = self
            .audio_graph
            .compile()
            .map_err(|e| AudioError::StreamError(format!("Graph compile error: {:?}", e)))?;

        let render_thread = crate::audio::render_thread::spawn_audio_render_thread(
            shared_state.clone(),
            compiled,
            message_rx,
            audio_queue.clone(),
            config.clone(),
            event_tx.clone(),
        );

        let callback = crate::audio::create_cpal_callback(audio_queue, shared_state.clone());

        let stream = device
            .build_output_stream(
                &cpal_config,
                callback,
                move |err| log::error!("Audio stream error: {}", err),
                None,
            )
            .map_err(|e| AudioError::StreamError(e.to_string()))?;

        stream
            .play()
            .map_err(|e| AudioError::StreamError(e.to_string()))?;

        event_tx.send(BackendEvent::Status(StatusEvent::AudioStarted {
            sample_rate,
        }));

        self.handle = Some(AudioHandle::new(render_thread, stream, shared_state));
        self.message_tx = Some(message_tx);

        Ok(event_rx)
    }

    /// Trigger note-on for the instrument at `instrument_idx`.
    pub fn note_on(
        &self,
        instrument_idx: usize,
        note: Note,
        velocity: f32,
    ) -> Result<(), AppError> {
        if !(0.0..=1.0).contains(&velocity) {
            return Err(AppError::InvalidParameter(format!(
                "velocity {velocity} out of range [0.0, 1.0]"
            )));
        }
        let source_index = self
            .audio_graph
            .source_map
            .get(&instrument_idx)
            .copied()
            .ok_or(AppError::InvalidInstrumentIndex)?;
        self.send_message(AudioMessage::Instrument(
            InstrumentAudioMessage::NoteStart {
                source_index,
                note,
                velocity,
            },
        ))
    }

    /// Trigger note-off for the instrument at `instrument_idx`.
    pub fn note_off(&self, instrument_idx: usize, note: Note) -> Result<(), AppError> {
        let source_index = self
            .audio_graph
            .source_map
            .get(&instrument_idx)
            .copied()
            .ok_or(AppError::InvalidInstrumentIndex)?;
        self.send_message(AudioMessage::Instrument(InstrumentAudioMessage::NoteStop {
            source_index,
            note,
        }))
    }

    /// Dispatch a frontend [`Command`].
    ///
    /// `AudioCommand`s are translated to source-index `AudioMessage`s internally.
    /// `GraphCommand`s mutate the visual graph and hot-swap the compiled result.
    pub fn send(&self, command: Command) -> Result<(), AppError> {
        match command {
            Command::Audio(AudioCommand::NoteStart {
                instrument_idx,
                note,
                velocity,
            }) => self.note_on(instrument_idx, note, velocity),

            Command::Audio(AudioCommand::NoteStop {
                instrument_idx,
                note,
            }) => self.note_off(instrument_idx, note),

            Command::Audio(AudioCommand::Shutdown) => self.send_message(AudioMessage::Shutdown),

            Command::Graph(cmd) => {
                let message_tx = self.message_tx.as_ref().ok_or(AppError::NotStarted)?;
                let sample_rate = self.config.system.sample_rate as f32;
                let mut gs = self.graph_system.lock().unwrap();
                handle_graph_command(cmd, &mut gs, sample_rate, message_tx)
            }

            Command::App(AppCommand::System(SystemCommand::Reset)) => {
                log::error!("Not implemented: System::Reset");
                Ok(())
            }
        }
    }

    /// Stop the engine: signal shutdown and join the render thread.
    pub fn stop(&mut self) -> Result<(), AppError> {
        if let Some(ref tx) = self.message_tx {
            let _ = tx.send(AudioMessage::Shutdown);
        }
        if let Some(handle) = self.handle.take() {
            handle
                .shutdown()
                .map_err(|e| AppError::AudioError(e.to_string()))?;
        }
        self.message_tx = None;
        Ok(())
    }

    /// Load configuration from a file.
    pub fn from_file(path: &Path) -> Result<App, AppError> {
        Ok(App {
            config: AppConfig::from_file(path)?,
            state: Arc::new(Mutex::new(AppState {
                mode: AppMode::Setup,
            })),
            audio_graph: AudioGraph::new(),
            graph_system: Mutex::new(GraphData::default()),
            handle: None,
            message_tx: None,
        })
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    fn send_message(&self, msg: AudioMessage) -> Result<(), AppError> {
        self.message_tx
            .as_ref()
            .ok_or(AppError::NotStarted)?
            .send(msg)
            .map_err(|_| AppError::ChannelClosed)
    }
}

impl Drop for App {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
