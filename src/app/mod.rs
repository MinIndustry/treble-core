//! Contains the application layer logic (configuration, thread managment)

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use log::info;

pub mod audio_graph;
pub mod commands;
mod config;
mod error;
mod filesystem;
pub(crate) mod graph_handler;
mod system;

use crate::app::audio_graph::AudioGraph;
use crate::app::error::AppError;
use crate::audio::EventSender;
use crate::audio::{
    AudioError, AudioHandle, AudioMessage, BackendEvent, EventFilter, EventReceiver,
    GraphAudioMessage, InstrumentAudioMessage, StatusEvent,
};
use crate::core::utils::Note;
use crate::instruments::Instrument;
use crate::instruments::registry::InstrumentRegistry;
use crate::instruments::spec::InstrumentSpec;
use std::collections::HashMap;

use commands::{AppCommand, AudioCommand, InstrumentCommand, SystemCommand};
use config::AppConfig;
use graph_handler::{GraphData, handle_graph_command};
use prelude::*;

// Export essential types directly from the app module
pub mod prelude {
    pub use super::App;
    pub use super::audio_graph::{
        AudioGraph, AudioGraphCompileError, AutomationSpec, AutomationTarget, BusSpec,
        InstrumentDefinition, InstrumentSlot, ParameterRamp,
    };
    pub use super::commands::{AppCommand, AudioCommand, Command};
    pub use super::filesystem::FSConfig;
    pub use super::system::SystemConfig;
    pub use crate::core::graph::RampCurve;
}

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

    /// All instrument slots. Populated before `start()`, compiled on start.
    pub audio_graph: AudioGraph,

    /// Instrument specs by name: built-ins + everything registered at runtime.
    pub registry: InstrumentRegistry,

    /// Slot indices of instantiated specs, by registry name.
    spec_slots: HashMap<String, usize>,

    /// Registry source name for each independently instantiated slot key.
    spec_sources: HashMap<String, String>,

    /// State for the visual graph editor.
    graph_system: GraphData,

    /// Handle to Treble's threads
    pub handle: Option<AudioHandle>,
    /// Direct channel to the render thread (no intermediate command thread).
    message_tx: Option<crossbeam::channel::Sender<AudioMessage>>,
    /// Backend event filter shared with the render thread.
    event_tx: Option<EventSender>,
    active_output_device: Option<String>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            config: AppConfig::default(),
            audio_graph: AudioGraph::new(),
            registry: InstrumentRegistry::built_in(),
            spec_slots: HashMap::new(),
            spec_sources: HashMap::new(),
            graph_system: GraphData::default(),
            handle: None,
            message_tx: None,
            event_tx: None,
            active_output_device: None,
        }
    }
}

impl App {
    pub fn new() -> App {
        App::default()
    }

    /// Initializes the application from the default configuration file (if any).
    /// If the configuration file is not found, returns an app with default parameters.
    pub fn init() -> Result<App, AppError> {
        if let Some(app_dirs) =
            directories::ProjectDirs::from(crate::APP_ID.2, crate::APP_ID.1, crate::APP_ID.0)
        {
            let config_path = app_dirs.config_dir().join("treble.toml");
            match App::from_file(&config_path) {
                Ok(app) => Ok(app),
                Err(AppError::FileNotFound) => {
                    log::info!(
                        "No config file found at {}, using defaults",
                        config_path.display()
                    );
                    Ok(App::default())
                }
                Err(e) => Err(e),
            }
        } else {
            log::warn!("Unable to build configuration directory, using defaults");
            Ok(App::default())
        }
    }

    /// Add an instrument and return its slot index (for use with `note_on`/`note_off`).
    ///
    /// Call before [`start()`](Self::start). To add instruments at runtime,
    /// call [`recompile()`](Self::recompile) afterwards.
    pub fn add_instrument(&mut self, instrument: Box<dyn Instrument>) -> usize {
        self.audio_graph.add_instrument(instrument)
    }

    /// Add an instrument from a spec (validated at this point, not at trigger
    /// time) and return its slot index. Like [`add_instrument`](Self::add_instrument),
    /// call [`recompile()`](Self::recompile) if the engine is already running.
    pub fn add_spec(&mut self, spec: InstrumentSpec) -> Result<usize, AppError> {
        self.audio_graph
            .add_spec(spec)
            .map_err(|e| AppError::InvalidParameter(e.to_string()))
    }

    /// Instantiate a registered spec by name: adds a slot for it and, when the
    /// engine is running, recompiles + hot-swaps. Idempotent — an already
    /// instantiated name just returns its existing slot index.
    pub fn instantiate(&mut self, name: &str) -> Result<usize, AppError> {
        self.instantiate_as(name, name)
    }

    /// Instantiate a registry spec into an independently addressable slot.
    /// Multiple keys may use the same spec without sharing mono voice state.
    pub fn instantiate_as(&mut self, instance: &str, spec_name: &str) -> Result<usize, AppError> {
        if let Some(&idx) = self.spec_slots.get(instance) {
            return Ok(idx);
        }
        let spec = self
            .registry
            .get(spec_name)
            .cloned()
            .ok_or_else(|| AppError::UnknownInstrument(spec_name.to_string()))?;
        let idx = self.add_spec(spec)?;
        self.spec_slots.insert(instance.to_string(), idx);
        self.spec_sources
            .insert(instance.to_string(), spec_name.to_string());
        if self.message_tx.is_some() {
            self.recompile()?;
        }
        Ok(idx)
    }

    /// Create or replace an independently addressed ephemeral spec slot.
    /// The definition is not added to the registry, making this suitable for
    /// editor previews of unsaved instrument JSON.
    pub fn instantiate_spec_as(
        &mut self,
        instance: &str,
        spec: InstrumentSpec,
    ) -> Result<usize, AppError> {
        let idx = self.instantiate_spec_as_deferred(instance, spec)?;
        if self.message_tx.is_some() {
            self.recompile()?;
        }
        Ok(idx)
    }

    /// Create or replace an ephemeral slot without immediately rebuilding the
    /// running graph. This allows callers to update a complete snapshot and
    /// perform one atomic recompile afterwards.
    pub fn instantiate_spec_as_deferred(
        &mut self,
        instance: &str,
        spec: InstrumentSpec,
    ) -> Result<usize, AppError> {
        let idx = if let Some(&idx) = self.spec_slots.get(instance) {
            self.audio_graph
                .replace_spec(idx, spec)
                .map_err(|error| AppError::InvalidParameter(error.to_string()))?;
            idx
        } else {
            let idx = self.add_spec(spec)?;
            self.spec_slots.insert(instance.to_string(), idx);
            idx
        };
        self.spec_sources.remove(instance);
        Ok(idx)
    }

    /// Register or replace a spec and rebuild every live instance that uses it.
    /// Register a spec and update any live instances, then rebuild the graph.
    ///
    /// The rebuild is immediate, which cuts sounding voices. Prefer
    /// [`Self::register_spec_deferred`] while audio is playing and schedule the
    /// rebuild on a cycle boundary instead.
    pub fn register_spec(&mut self, spec: InstrumentSpec) -> Result<(), AppError> {
        self.register_spec_deferred(spec)?;
        if self.message_tx.is_some() {
            self.recompile()?;
        }
        Ok(())
    }

    /// Register a spec and update any live instances **without** rebuilding the
    /// graph, leaving the caller to choose when that happens.
    ///
    /// A rebuild mid-cycle silences whatever is sounding until the next one, so
    /// a caller that is already scheduling a boundary swap wants this and its
    /// own `recompile_at`.
    pub fn register_spec_deferred(&mut self, spec: InstrumentSpec) -> Result<(), AppError> {
        let name = spec.name.clone();
        self.registry
            .register(spec.clone())
            .map_err(|error| AppError::InvalidParameter(error.to_string()))?;
        let slots: Vec<usize> = self
            .spec_sources
            .iter()
            .filter_map(|(instance, source)| {
                (source == &name)
                    .then(|| self.spec_slots.get(instance).copied())
                    .flatten()
            })
            .collect();
        for slot in slots {
            self.audio_graph
                .replace_spec(slot, spec.clone())
                .map_err(|error| AppError::InvalidParameter(error.to_string()))?;
        }
        Ok(())
    }

    /// Slot index of an instantiated spec, by registry name.
    pub fn instrument_idx(&self, name: &str) -> Option<usize> {
        self.spec_slots.get(name).copied()
    }

    /// Replace the shared bus chains applied at the next graph build.
    ///
    /// Members are instance names as passed to the register calls; names with
    /// no live slot are skipped. Takes effect on the next `recompile*`.
    pub fn set_buses(
        &mut self,
        buses: Vec<(String, Vec<crate::instruments::spec::FxSpec>, Vec<String>)>,
    ) {
        let resolved = buses
            .into_iter()
            .map(|(name, fx, members)| crate::app::audio_graph::BusSpec {
                name,
                fx,
                members: members
                    .iter()
                    .filter_map(|member| self.spec_slots.get(member).copied())
                    .collect(),
            })
            .collect();
        self.audio_graph.set_buses(resolved);
    }

    /// Declare a parameter sweep on a filter in an instrument instance's own
    /// fx chain. `fx_index` indexes the instance's `InstrumentSpec::fx`.
    ///
    /// The sweep is stored as declarative data and re-applied by every
    /// `recompile*`, so it survives the rebuild a live edit causes. Its frames
    /// are absolute engine frames (see [`current_frame`](Self::current_frame)),
    /// which is what lets the replacement filter continue the ramp instead of
    /// restarting it. Takes effect at the next graph build.
    pub fn automate_instrument_fx(
        &mut self,
        instance: &str,
        fx_index: usize,
        ramp: ParameterRamp,
    ) -> Result<(), AppError> {
        let slot = self
            .instrument_idx(instance)
            .ok_or_else(|| AppError::UnknownInstrument(instance.to_string()))?;
        self.audio_graph.add_automation(AutomationSpec {
            target: AutomationTarget::InstrumentFx { slot, fx_index },
            ramp,
        });
        Ok(())
    }

    /// Declare a parameter sweep on a filter in a named bus chain, as passed to
    /// [`set_buses`](Self::set_buses). A name or index that the compiled graph
    /// has no filter for is dropped at build time, like an out-of-range bus
    /// member, so bus and sweep declarations may arrive in either order.
    pub fn automate_bus_fx(&mut self, bus: &str, fx_index: usize, ramp: ParameterRamp) {
        self.audio_graph.add_automation(AutomationSpec {
            target: AutomationTarget::BusFx {
                bus: bus.to_string(),
                fx_index,
            },
            ramp,
        });
    }

    /// Replace the whole declared sweep set, in the graph's own addressing.
    pub fn set_automations(&mut self, automations: Vec<AutomationSpec>) {
        self.audio_graph.set_automations(automations);
    }

    /// Drop every declared sweep. Callers that re-declare their whole set per
    /// evaluation clear first, mirroring how they replace the bus set.
    pub fn clear_automations(&mut self) {
        self.audio_graph.clear_automations();
    }

    /// The declared sweeps, in declaration order.
    pub fn automations(&self) -> &[AutomationSpec] {
        self.audio_graph.automations()
    }

    /// Recompile the audio graph and hot-swap it into the running render thread.
    pub fn recompile(&mut self) -> Result<(), AppError> {
        let sample_rate = self.config.system.sample_rate as f32;
        let system = self
            .audio_graph
            .compile(sample_rate)
            .map_err(|e| AppError::AudioError(format!("{:?}", e)))?;
        self.send_message(AudioMessage::Graph(GraphAudioMessage::Swap(system)))
    }

    /// Recompile now, but install the resulting graph at an exact engine frame.
    pub fn recompile_at(&mut self, at_frame: u64) -> Result<(), AppError> {
        let sample_rate = self.config.system.sample_rate as f32;
        let system = self
            .audio_graph
            .compile(sample_rate)
            .map_err(|e| AppError::AudioError(format!("{:?}", e)))?;
        self.send_message(AudioMessage::ScheduledGraphSwap {
            at_frame,
            system,
            fade_in_frames: (sample_rate * 0.008).round() as u64,
            tail_frames: (sample_rate * 0.75).round() as u64,
        })
    }

    /// Start the audio engine.
    ///
    /// Compiles the instrument graph, spawns the render thread, and returns
    /// a receiver for backend events. Pass an [`EventFilter`] to control which
    /// event categories are forwarded; use [`EventFilter::all()`] to receive everything.
    pub fn start(&mut self, filter: EventFilter) -> Result<EventReceiver, AudioError> {
        self.start_on_output_device(filter, None)
    }

    /// Start on a named output device, or the operating-system default when omitted.
    pub fn start_on_output_device(
        &mut self,
        filter: EventFilter,
        output_device: Option<&str>,
    ) -> Result<EventReceiver, AudioError> {
        if self.handle.is_some() {
            return Err(AudioError::ConfigError(
                "audio engine is already running".into(),
            ));
        }
        let (event_tx, event_rx) =
            EventSender::new(filter, self.config.audio.audio_event_queue_size);

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
        let engine_sample_rate = self.config.system.sample_rate;
        shared_state
            .engine_sample_rate
            .store(engine_sample_rate, Ordering::Relaxed);
        shared_state
            .master_volume
            .store(self.config.system.master_volume, Ordering::Relaxed);

        let compiled = self
            .audio_graph
            .compile(engine_sample_rate as f32)
            .map_err(|e| AudioError::StreamError(format!("Graph compile error: {:?}", e)))?;

        let (output_stream, stream_info) = crate::audio::stream::spawn_output_stream(
            output_device.map(str::to_owned),
            audio_queue.clone(),
            shared_state.clone(),
            config.clone(),
            event_tx.clone(),
        )?;
        info!(
            "Audio config: device='{}', engine_rate={engine_sample_rate}, output_rate={}, buffer_size={}, ring_buffer={}",
            stream_info.device_name,
            stream_info.sample_rate,
            config.cpal_buffer_size,
            config.audio_ring_buffer_size
        );

        let render_thread = crate::audio::render_thread::spawn_audio_render_thread(
            shared_state.clone(),
            compiled,
            message_rx,
            audio_queue.clone(),
            config.clone(),
            event_tx.clone(),
        )
        .map_err(|error| {
            AudioError::StreamError(format!(
                "TRBC-RT-002: could not spawn the audio render thread: {error}"
            ))
        })?;

        event_tx.send(BackendEvent::Status(StatusEvent::AudioStarted {
            sample_rate: stream_info.sample_rate,
        }));
        event_tx.send(BackendEvent::Status(StatusEvent::OutputDeviceChanged {
            device: stream_info.device_name.clone(),
        }));

        self.handle = Some(AudioHandle::new(render_thread, output_stream, shared_state));
        self.active_output_device = Some(stream_info.device_name);
        self.message_tx = Some(message_tx);
        self.event_tx = Some(event_tx);

        Ok(event_rx)
    }

    /// Enumerate output devices visible to the current CPAL host.
    pub fn output_devices() -> Result<Vec<String>, AudioError> {
        crate::audio::output_device_names()
    }

    pub fn active_output_device(&self) -> Option<&str> {
        self.active_output_device.as_deref()
    }

    /// Change which backend event categories are forwarded without restarting audio.
    pub fn set_event_filter(&self, filter: EventFilter) {
        if let Some(sender) = &self.event_tx {
            sender.set_filter(filter);
        }
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

    /// Schedule note-on for an absolute engine frame (sample-accurate).
    ///
    /// Compute `at_frame` from [`current_frame()`](Self::current_frame) plus a
    /// lookahead. Frames already in the past apply at the next block start.
    pub fn note_on_at(
        &self,
        instrument_idx: usize,
        note: Note,
        velocity: f32,
        at_frame: u64,
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
        self.send_message(AudioMessage::ScheduledInstrument {
            at_frame,
            command: InstrumentAudioMessage::NoteStart {
                source_index,
                note,
                velocity,
            },
        })
    }

    /// Schedule note-off for an absolute engine frame (sample-accurate).
    pub fn note_off_at(
        &self,
        instrument_idx: usize,
        note: Note,
        at_frame: u64,
    ) -> Result<(), AppError> {
        let source_index = self
            .audio_graph
            .source_map
            .get(&instrument_idx)
            .copied()
            .ok_or(AppError::InvalidInstrumentIndex)?;
        self.send_message(AudioMessage::ScheduledInstrument {
            at_frame,
            command: InstrumentAudioMessage::NoteStop { source_index, note },
        })
    }

    /// The engine frame clock: total frames rendered since `start()`.
    /// `None` before the engine is started.
    pub fn current_frame(&self) -> Option<u64> {
        self.handle
            .as_ref()
            .map(|h| h.shared_state().current_frame.load(Ordering::Relaxed))
    }

    /// Current device rate and callback underrun count, if audio is running.
    pub fn audio_metrics(&self) -> Option<crate::audio::AudioMetrics> {
        self.handle.as_ref().map(AudioHandle::get_metrics)
    }

    /// Dispatch a frontend [`Command`].
    ///
    /// `AudioCommand`s are translated to source-index `AudioMessage`s internally.
    /// `GraphCommand`s mutate the visual graph and hot-swap the compiled result.
    /// `InstrumentCommand`s manage the spec registry and instrument slots.
    pub fn send(&mut self, command: Command) -> Result<(), AppError> {
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
                handle_graph_command(cmd, &mut self.graph_system, sample_rate, message_tx)
            }

            Command::Instrument(InstrumentCommand::Register(spec)) => self
                .registry
                .register(spec)
                .map_err(|e| AppError::InvalidParameter(e.to_string())),

            Command::Instrument(InstrumentCommand::Unregister { name }) => {
                if self.registry.unregister(&name) {
                    Ok(())
                } else {
                    Err(AppError::UnknownInstrument(name))
                }
            }

            Command::Instrument(InstrumentCommand::Instantiate { name }) => {
                self.instantiate(&name).map(|_| ())
            }

            Command::App(AppCommand::System(SystemCommand::SetMasterVolume(vol))) => {
                self.handle
                    .as_ref()
                    .ok_or(AppError::NotStarted)?
                    .shared_state()
                    .master_volume
                    .store(vol, Ordering::Relaxed);
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
        self.event_tx = None;
        self.active_output_device = None;
        Ok(())
    }

    /// Load configuration from a file.
    pub fn from_file(path: &Path) -> Result<App, AppError> {
        let mut app = App::default();
        app.config = AppConfig::from_file(path)?;
        Ok(app)
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
