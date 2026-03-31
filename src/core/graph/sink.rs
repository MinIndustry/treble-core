use crate::core::graph::Entry;
use crate::core::{Block, Frame};

/// A trait for AudioGraphElements that allow other parts of the
/// code to consume values from them. (Acts as a graph output)
pub trait Sink: Entry + Send + Sync {
    /// Gets the values of the sink
    fn consume(&mut self) -> Block;
    fn get_frames(&self) -> &[Frame];
    fn into_entry(self) -> Box<dyn Entry>;
    /// Update a named parameter. Default implementation is a no-op.
    fn set_parameter(&mut self, _name: &str, _value: f32) {}
}
dyn_clone::clone_trait_object!(Sink);
