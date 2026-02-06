//! Graphs are used to build audio pipelines for the application.
//! They serve as metastructures for filters.
//! Graphs can be used to create complex audio effects and processing chains.
use dyn_clone::DynClone;
use std::fmt;

/// An element in the Audio pipeline
/// Has a name and an index. Is able to connect to other elements
pub trait AudioGraphElement: std::fmt::Debug {
    /// Connects this element to another element.
    /// The to element must implement the Entry trait.
    fn get_name(&self) -> &str;
    fn get_index(&self) -> usize;
    fn set_index(&mut self, index: usize);
}

/// A trait for AudioGraphElements to allow other elements to push
/// values into them.
pub trait Entry: AudioGraphElement + DynClone {
    /// Pushes a value into this element (sink or filter)
    fn push(&mut self, value: f32, port: usize);
}

/// A trait for AudioGraphElements that allow neighbours in the graph to pull
/// values from them. (Acts as a graph input)
pub trait Source: AudioGraphElement {
    /// Pull the next input from the source
    fn pull(&mut self) -> f32;
}

/// A trait for AudioGraphElements that allow other parts of the
/// code to consume values from them. (Acts as a graph output)
pub trait Sink: Entry + AudioGraphElement {
    /// Gets the values of the sink
    fn consume(&mut self, amount: usize) -> Vec<f32>;
    fn get_values(&self) -> Vec<f32>;
    fn into_entry(self) -> Box<dyn Entry>;
}

/// A filter that can process data. Data should be pushed to the filter's input by either the preceding filter or a source.
pub trait Filter: Entry + AudioGraphElement + fmt::Display + fmt::Debug {
    /// Applies the filter's transformation to the input
    /// Returns a tuple of the output and the indices of the elements that the filter is connected to
    fn transform(&mut self) -> Vec<f32>;

    /// Returns true if the filter's execution can be postponed to the end of the execution cycle of the graph.
    /// An postonable element must be present in a cycle of the graph to avoid infinite looping.
    /// E.g. a delay filter can be postponed if it lies within a feedback loop.
    fn postponable(&self) -> bool;

    /// Enables downcasting from trait object to concrete type.
    /// This is required to access type-specific methods (like reset()) that aren't part of the Filter trait.
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// The sink module provides implementations for various types of sinks.
mod sink;
/// The source module provides implementations for various types of sources.
mod source;
mod system;

pub use sink::SimpleSink;
pub use source::{simple_source, SimpleSource};

/// The system module contains the implementation of the system element.
pub use system::System;
