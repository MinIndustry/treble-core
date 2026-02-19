//! Graphs are used to build audio pipelines for the application.
//! They serve as metastructures for filters.
//! Graphs can be used to create complex audio effects and processing chains.

mod audio_sink;
mod error;
/// The sink module provides implementations for various types of sinks.
mod simple_sink;
/// The source module provides implementations for various types of sources.
mod simple_source;
mod system;
mod source;
mod filter;
mod sink;
mod entry;

pub use audio_sink::AudioOutputSink;

pub use source::Source;
pub use filter::Filter;
pub use sink::Sink;
pub use entry::Entry;

pub use simple_sink::SimpleSink;
pub use simple_source::{SimpleSource, simple_source};

/// The system module contains the implementation of the system element.
pub use system::System;
