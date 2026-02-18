//! Graphs are used to build audio pipelines for the application.
//! They serve as metastructures for filters.
//! Graphs can be used to create complex audio effects and processing chains.
use dyn_clone::DynClone;
use std::fmt;

/// A trait to allow other elements to push
/// values into them.
pub trait Entry: fmt::Debug + DynClone + Send {
    /// Pushes a value into this element (sink or filter)
    fn push(&mut self, value: f32, port: usize);
}

/// A trait for AudioGraphElements that allow neighbours in the graph to pull
/// values from them. (Acts as a graph input)
pub trait Source: fmt::Debug + DynClone + Send {
    /// Pull the next input from the source
    fn pull(&mut self) -> f32;
    /// Start the source (e.g. note-on)
    fn start(&mut self) {}
    /// Stop the source (e.g. note-off)
    fn stop(&mut self) {}
    /// Whether this source is currently producing signal
    fn is_active(&self) -> bool {
        true
    }
}
dyn_clone::clone_trait_object!(Source);

/// A trait for AudioGraphElements that allow other parts of the
/// code to consume values from them. (Acts as a graph output)
pub trait Sink: Entry {
    /// Gets the values of the sink
    fn consume(&mut self, amount: usize) -> Vec<f32>;
    fn get_values(&self) -> Vec<f32>;
    fn into_entry(self) -> Box<dyn Entry>;
}
dyn_clone::clone_trait_object!(Sink);

/// A filter that can process data. Data should be pushed to the filter's input by either the preceding filter or a source.
/// When the `meta` feature is enabled, filters must also implement `MetaFilter` for named parameter access.
#[cfg(feature = "meta")]
pub trait Filter: Entry + fmt::Display + rustic_meta::MetaFilter {
    /// Applies the filter's transformation to the input
    /// Returns a tuple of the output and the indices of the elements that the filter is connected to
    fn transform(&mut self) -> Vec<f32>;

    /// Returns true if the filter's execution can be postponed to the end of the execution cycle of the graph.
    /// A postponable element must be present in a cycle of the graph to avoid infinite looping.
    /// E.g. a delay filter can be postponed if it lies within a feedback loop.
    fn postponable(&self) -> bool {
        false
    }

    /// Enables downcasting from trait object to concrete type.
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// A filter that can process data. Data should be pushed to the filter's input by either the preceding filter or a source.
#[cfg(not(feature = "meta"))]
pub trait Filter: Entry + fmt::Display {
    /// Applies the filter's transformation to the input
    /// Returns a tuple of the output and the indices of the elements that the filter is connected to
    fn transform(&mut self) -> Vec<f32>;

    /// Returns true if the filter's execution can be postponed to the end of the execution cycle of the graph.
    /// A postponable element must be present in a cycle of the graph to avoid infinite looping.
    /// E.g. a delay filter can be postponed if it lies within a feedback loop.
    fn postponable(&self) -> bool {
        false
    }

    /// Enables downcasting from trait object to concrete type.
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}
dyn_clone::clone_trait_object!(Filter);

mod audio_sink;
mod error;
/// The sink module provides implementations for various types of sinks.
mod sink;
/// The source module provides implementations for various types of sources.
mod source;
mod system;

pub use audio_sink::AudioOutputSink;
pub use sink::SimpleSink;
pub use source::{SimpleSource, simple_source};

/// The system module contains the implementation of the system element.
pub use system::System;
