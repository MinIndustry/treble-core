use super::{ModuleRegistry, PreparedPatch, compile::Binding, error};
use crate::core::{Note, graph::System};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use treble_meta::{
    InstrumentDocument, ParameterValue, UpdateMode, ValidationErrors, VoiceConfiguration,
};

/// Host-owned editable document. Keep one session for one playback runtime.
/// Saving `document()` is independent of successful preparation or installation.
pub struct PatchSession {
    registry: ModuleRegistry,
    document: InstrumentDocument,
    revision: u64,
    owner: Arc<AtomicU64>,
    active: Option<ActivePatch>,
}

#[derive(Clone)]
struct ActivePatch {
    generation: u64,
    document: InstrumentDocument,
    bindings: BTreeMap<String, Binding>,
    slots: BTreeMap<(String, String), usize>,
    note_routes: BTreeMap<String, Vec<usize>>,
}

/// Owned snapshot suitable for background preparation while the draft changes.
#[derive(Clone)]
pub struct PatchSnapshot {
    registry: ModuleRegistry,
    document: InstrumentDocument,
    revision: u64,
    owner: Arc<AtomicU64>,
}

/// Prepared snapshot with origin identity and revision, checked again at commit.
pub struct PreparedEdit {
    patch: PreparedPatch,
    revision: u64,
    owner: Arc<AtomicU64>,
}

impl PatchSnapshot {
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn prepare(
        self,
        sample_rate: f32,
        block_size: usize,
    ) -> Result<PreparedEdit, ValidationErrors> {
        let patch = self
            .registry
            .prepare(&self.document, sample_rate, block_size)?;
        Ok(PreparedEdit {
            patch,
            revision: self.revision,
            owner: self.owner,
        })
    }
}

/// A live update carries a command; rebuild updates only change the draft.
pub struct ParameterEdit {
    pub update: UpdateMode,
    pub command: Option<PatchCommand>,
}

/// Validated control for the existing renderer. Fields are deliberately private:
/// create controls through `PatchSession`, not by supplying unchecked indices.
#[derive(Clone, Debug)]
pub struct PatchCommand {
    owner: Arc<AtomicU64>,
    generation: u64,
    kind: CommandKind,
}

#[derive(Clone, Debug)]
enum CommandKind {
    Install {
        patch: Box<PreparedPatch>,
        parameter_versions: Vec<u64>,
    },
    Parameter {
        binding: Binding,
        name: &'static str,
        value: f32,
        slot: usize,
        revision: u64,
    },
    NoteOn {
        sources: Vec<usize>,
        note: Note,
        velocity: f32,
    },
    NoteOff {
        sources: Vec<usize>,
        note: Note,
    },
    Kill {
        sources: Vec<usize>,
    },
}

impl PatchCommand {
    /// Engine sample rate required by a prepared installation, if any.
    pub fn installation_sample_rate(&self) -> Option<f32> {
        match &self.kind {
            CommandKind::Install { patch, .. } => Some(patch.sample_rate),
            _ => None,
        }
    }
}

impl PatchSession {
    pub fn new(
        registry: ModuleRegistry,
        document: InstrumentDocument,
    ) -> Result<Self, ValidationErrors> {
        document.validate()?;
        Ok(Self {
            registry,
            document,
            revision: 1,
            owner: Arc::new(AtomicU64::new(0)),
            active: None,
        })
    }
    pub fn document(&self) -> &InstrumentDocument {
        &self.document
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    /// Accept a saveable draft even if installed modules/cables cannot play it.
    pub fn replace_document(
        &mut self,
        document: InstrumentDocument,
    ) -> Result<(), ValidationErrors> {
        document.validate()?;
        self.bump()?;
        self.document = document;
        Ok(())
    }
    fn bump(&mut self) -> Result<(), ValidationErrors> {
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or_else(|| error("revision", "session revision exhausted"))?;
        Ok(())
    }
    pub fn snapshot(&self) -> PatchSnapshot {
        PatchSnapshot {
            registry: self.registry.clone(),
            document: self.document.clone(),
            revision: self.revision,
            owner: self.owner.clone(),
        }
    }
    /// Accept only a current snapshot of this session. Queue the returned command
    /// exactly once. Keep/retry it if the host's bounded audio queue is full.
    pub fn commit(&mut self, prepared: PreparedEdit) -> Result<PatchCommand, ValidationErrors> {
        if !Arc::ptr_eq(&prepared.owner, &self.owner) || prepared.revision != self.revision {
            return Err(error(
                "revision",
                "prepared patch is stale or belongs to another session; prepare the current document",
            ));
        }
        let slots: BTreeMap<_, _> = prepared
            .patch
            .document
            .instrument
            .patch
            .modules
            .iter()
            .flat_map(|module| {
                self.registry
                    .descriptor(&module.type_id, module.type_version)
                    .into_iter()
                    .flat_map(move |descriptor| {
                        descriptor
                            .parameters
                            .iter()
                            .map(move |parameter| (module.id.clone(), parameter.id.clone()))
                    })
            })
            .enumerate()
            .map(|(index, key)| (key, index))
            .collect();
        let parameter_versions = vec![0; slots.len()];
        self.active = Some(ActivePatch {
            generation: self.revision,
            document: self.document.clone(),
            bindings: prepared.patch.bindings.clone(),
            slots,
            note_routes: prepared.patch.note_routes.clone(),
        });
        self.owner.store(self.revision, Ordering::Release);
        Ok(PatchCommand {
            owner: self.owner.clone(),
            generation: self.revision,
            kind: CommandKind::Install {
                patch: Box::new(prepared.patch),
                parameter_versions,
            },
        })
    }

    pub fn set_parameter(
        &mut self,
        module_id: &str,
        id: &str,
        value: ParameterValue,
    ) -> Result<ParameterEdit, ValidationErrors> {
        let index = self
            .document
            .instrument
            .patch
            .modules
            .iter()
            .position(|m| m.id == module_id)
            .ok_or_else(|| error(format!("modules.{module_id}"), "unknown module instance"))?;
        let instance = &self.document.instrument.patch.modules[index];
        let descriptor = self
            .registry
            .descriptor(&instance.type_id, instance.type_version)
            .ok_or_else(|| {
                error(
                    format!("modules.{module_id}"),
                    "module type/version is not installed",
                )
            })?;
        let parameter = descriptor
            .parameters
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| {
                error(
                    format!("modules.{module_id}.parameters.{id}"),
                    "unknown parameter",
                )
            })?;
        parameter.validate_value(&super::modules::runtime_value(&value))?;
        let declared_update = parameter.update.clone();
        let mapping = scalar_binding(&instance.type_id, id);
        let active = self
            .active
            .as_ref()
            .filter(|active| same_structure(&active.document, &self.document));
        let command = if declared_update == UpdateMode::Live {
            active.zip(mapping).map(|(active, name)| {
                let scalar = match &value {
                    ParameterValue::Float(x) => *x as f32,
                    ParameterValue::Int(x) => *x as f32,
                    ParameterValue::Bool(x) => {
                        if *x {
                            1.0
                        } else {
                            0.0
                        }
                    }
                    ParameterValue::Enum(_) => unreachable!("live builtins have scalar values"),
                };
                PatchCommand {
                    owner: self.owner.clone(),
                    generation: active.generation,
                    kind: CommandKind::Parameter {
                        binding: active.bindings[module_id],
                        name,
                        value: scalar,
                        slot: active.slots[&(module_id.to_owned(), id.to_owned())],
                        revision: self.revision + 1,
                    },
                }
            })
        } else {
            None
        };
        self.bump()?;
        self.document.instrument.patch.modules[index]
            .parameters
            .insert(id.into(), value);
        // Keep base values in both draft and active blueprint. This is required
        // for subsequent structural comparison and persistence after live edits.
        if command.is_some()
            && let Some(active) = &mut self.active
        {
            active.document = self.document.clone();
        }
        let update = if command.is_some() {
            UpdateMode::Live
        } else {
            UpdateMode::Rebuild
        };
        Ok(ParameterEdit { update, command })
    }

    fn command(&self, kind: CommandKind) -> Result<PatchCommand, ValidationErrors> {
        let active = self
            .active
            .as_ref()
            .ok_or_else(|| error("playback", "commit a prepared patch before playing"))?;
        Ok(PatchCommand {
            owner: self.owner.clone(),
            generation: active.generation,
            kind,
        })
    }
    fn note_sources(&self) -> Result<Vec<usize>, ValidationErrors> {
        let active = self
            .active
            .as_ref()
            .ok_or_else(|| error("playback", "commit a prepared patch before playing"))?;
        if !matches!(
            active.document.instrument.voice,
            VoiceConfiguration::Monophonic
        ) {
            return Err(error(
                "instrument.voice",
                "note events require monophonic voice configuration",
            ));
        }
        let input = active
            .document
            .instrument
            .interface
            .note_input
            .as_ref()
            .ok_or_else(|| {
                error(
                    "instrument.interface.note_input",
                    "instrument has no exposed note input",
                )
            })?;
        match active.note_routes.get(&input.module) {
            Some(sources) => Ok(sources.clone()),
            _ => Err(error(
                "instrument.interface.note_input",
                "note input is not bound to a playable source",
            )),
        }
    }
    pub fn note_on(&self, note: Note, velocity: f32) -> Result<PatchCommand, ValidationErrors> {
        if !velocity.is_finite() || !(0.0..=1.0).contains(&velocity) {
            return Err(error("velocity", "expected a finite value in 0..=1"));
        }
        self.command(CommandKind::NoteOn {
            sources: self.note_sources()?,
            note,
            velocity,
        })
    }
    pub fn note_off(&self, note: Note) -> Result<PatchCommand, ValidationErrors> {
        self.command(CommandKind::NoteOff {
            sources: self.note_sources()?,
            note,
        })
    }
    /// Immediately stop every source, including continuous drones.
    pub fn all_notes_off(&self) -> Result<PatchCommand, ValidationErrors> {
        let active = self
            .active
            .as_ref()
            .ok_or_else(|| error("playback", "no committed patch"))?;
        let sources = active
            .bindings
            .values()
            .filter_map(|binding| match binding {
                Binding::Source(i) => Some(*i),
                _ => None,
            })
            .collect();
        self.command(CommandKind::Kill { sources })
    }
}

fn same_structure(a: &InstrumentDocument, b: &InstrumentDocument) -> bool {
    a.instrument.patch.cables == b.instrument.patch.cables
        && a.instrument.interface == b.instrument.interface
        && a.instrument.voice == b.instrument.voice
        && a.instrument.patch.modules.len() == b.instrument.patch.modules.len()
        && a.instrument
            .patch
            .modules
            .iter()
            .zip(&b.instrument.patch.modules)
            .all(|(a, b)| {
                // Rebuild-only values matter; a live update cannot pretend a pending
                // waveform or delay-size edit was installed.
                a.id == b.id
                    && a.type_id == b.type_id
                    && a.type_version == b.type_version
                    && a.parameters == b.parameters
            })
}

pub(crate) fn scalar_binding(type_id: &str, parameter: &str) -> Option<&'static str> {
    match (type_id, parameter) {
        ("treble.lfo", "frequency") => Some("frequency"),
        ("treble.lfo", "amplitude") => Some("amplitude"),
        ("treble.oscillator", "frequency") => Some("frequency"),
        ("treble.oscillator", "amplitude") => Some("amplitude"),
        ("treble.oscillator", "follow_keys") => Some("follow_keys"),
        ("treble.low_pass", "cutoff") => Some("cutoff_frequency"),
        ("treble.low_pass", "resonance") => Some("resonance"),
        ("treble.delay", "feedback") => Some("feedback"),
        ("treble.delay", "mix") => Some("mix"),
        ("treble.output", "level") => Some("factor"),
        _ => None,
    }
}

/// Control state beside the existing System, shared by native and headless hosts.
/// Call `apply` between rendered blocks. It never compiles a graph.
#[derive(Default)]
pub struct PatchRuntime {
    active: Option<RuntimeGeneration>,
}
struct RuntimeGeneration {
    owner: Arc<AtomicU64>,
    generation: u64,
    parameter_versions: Vec<u64>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PatchApply {
    Installed,
    Applied,
    Superseded,
}

impl PatchRuntime {
    pub fn apply(
        &mut self,
        system: &mut System,
        command: PatchCommand,
    ) -> Result<PatchApply, ValidationErrors> {
        if command.owner.load(Ordering::Acquire) != command.generation {
            return Ok(PatchApply::Superseded);
        }
        if let CommandKind::Install {
            patch,
            parameter_versions,
        } = command.kind
        {
            if let Some(active) = &self.active {
                if !Arc::ptr_eq(&active.owner, &command.owner) {
                    return Err(error(
                        "playback",
                        "runtime belongs to another session; reset the runtime before attaching a different session",
                    ));
                }
                if active.generation >= command.generation {
                    return Ok(PatchApply::Superseded);
                }
            }
            *system = patch.system;
            self.active = Some(RuntimeGeneration {
                owner: command.owner,
                generation: command.generation,
                parameter_versions,
            });
            return Ok(PatchApply::Installed);
        }
        let Some(active) = &mut self.active else {
            return Ok(PatchApply::Superseded);
        };
        if !Arc::ptr_eq(&active.owner, &command.owner) || active.generation != command.generation {
            return Ok(PatchApply::Superseded);
        }
        match command.kind {
            CommandKind::Parameter {
                binding,
                name,
                value,
                slot,
                revision,
            } => {
                let version = &mut active.parameter_versions[slot];
                if revision <= *version {
                    return Ok(PatchApply::Superseded);
                }
                match binding {
                    Binding::Source(index) => system
                        .set_source_parameter(index, name, value)
                        .map_err(|e| error("parameter", e.to_string()))?,
                    Binding::NoteInput => {
                        return Err(error("parameter", "note router has no parameters"));
                    }
                    Binding::Filter(index) => {
                        let filter = system
                            .get_filter_mut(index)
                            .ok_or_else(|| error("parameter", "compiled filter is missing"))?;
                        if !filter.set_parameter(name, value) {
                            return Err(error(
                                "parameter",
                                "compiled filter refused registered parameter",
                            ));
                        }
                    }
                }
                *version = revision;
            }
            CommandKind::NoteOn {
                sources,
                note,
                velocity,
            } => {
                for source in sources {
                    system.start_note(source, note, velocity);
                }
            }
            CommandKind::NoteOff { sources, note } => {
                for source in sources {
                    system.stop_note(source, note);
                }
            }
            CommandKind::Kill { sources } => {
                for source in sources {
                    system.kill_source(source);
                }
            }
            CommandKind::Install { .. } => unreachable!(),
        }
        Ok(PatchApply::Applied)
    }
}
