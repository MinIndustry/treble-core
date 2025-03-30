use std::fmt;
use dyn_clone::DynClone;

/// An element in the Audio pipeline
/// Has a name and uuid. Is able to connect to other elements
pub trait AudioGraphElement {
    /// Connects this element to another element.
    /// The to element must implement the Entry trait.
    fn get_name(&self) -> &str;
    fn get_index(&self) -> usize;
    fn set_index(&mut self, index: usize);
}

/// A trait that allows an element to be pushed to
pub trait Entry: AudioGraphElement + DynClone {
    /// Pushes a value into this element (sink or filter)
    fn push(&mut self, value: f32, port: usize);
}

pub trait Source: AudioGraphElement {
    /// Pull the next input from the source
    fn pull(&mut self) -> f32;
}

pub trait Sink: Entry + AudioGraphElement {
    /// Gets the values of the sink
    fn consume(&mut self, amount: usize) -> Vec<f32>;
    fn get_values(&self) -> Vec<f32>;
    fn as_entry(self) -> Box<dyn Entry>;
}

/// A filter that can process data. Data should be pushed to the filter's input by either the preceding filter or a source.
pub trait Filter: Entry + AudioGraphElement + fmt::Display + fmt::Debug {
    /// Applies the filter's transformation to the input
    /// Returns a tuple of the output and the indices of the elements that the filter is connected to
    fn transform(&mut self) -> Vec<f32>;
    fn postponable(&self) -> bool;
}

mod system;
mod sink;
mod source;

pub use sink::SimpleSink;
pub use source::{SimpleSource, simple_source};

pub use system::System;
