//! Graphs are used to build audio pipelines for the application.
//! They serve as metastructures for filters.
//! Graphs can be used to create complex audio effects and processing chains.

mod audio_sink;
mod entry;
mod error;
mod filter;
/// The sink module provides implementations for various types of sinks.
mod simple_sink;
/// The source module provides implementations for various types of sources.
mod simple_source;
mod sink;
mod source;
mod system;

pub use audio_sink::AudioOutputSink;

pub use entry::Entry;
pub use filter::Filter;
pub use sink::Sink;
pub use source::Source;

pub use simple_sink::SimpleSink;
pub use simple_source::{SimpleSource, simple_source};

/// The system module contains the implementation of the system element.
pub use system::System;
