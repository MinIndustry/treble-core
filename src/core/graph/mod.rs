//! Graphs are used to build audio pipelines for the application.
//! They serve as metastructures for filters.
//! Graphs can be used to create complex audio effects and processing chains.

mod audio_node;
mod audio_sink;
/// Block-rate parameter sweeps applied by the render loop.
mod automation;
mod entry;
pub mod error;
mod filter;
mod modulation;
mod note_lifecycle;
pub(crate) use modulation::{ExecutionNode, SamplePlan, ScheduledNode, SignalRoute, SourceRoute};
pub use modulation::{ParameterModulation, modulated_value};
/// The sink module provides implementations for various types of sinks.
mod simple_sink;
/// The source module provides implementations for various types of sources.
mod simple_source;
mod sink;
mod source;
/// Monophonic and polyphonic Source implementations.
pub mod sources;
mod system;

pub use audio_sink::AudioOutputSink;
pub use automation::{ParameterAutomation, RampCurve};
pub use error::AudioGraphError;

pub use entry::Entry;
pub use filter::Filter;
pub use note_lifecycle::NoteLifecycleSource;
pub use sink::{Sink, SinkTelemetry};
pub use source::{Source, SourceInput};

pub use simple_sink::SimpleSink;
pub use simple_source::{SimpleSource, simple_source};
pub use sources::{
    MonophonicAllocationStrategy, MonophonicSource, PolyphonicAllocationStrategy, PolyphonicSource,
};

/// The system module contains the implementation of the system element.
pub use system::{ModTarget, ModWire, RunStats, System};
