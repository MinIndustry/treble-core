use crate::core::graph::Entry;
use crate::core::{Block, Frame};

/// A trait for AudioGraphElements that allow other parts of the
/// code to consume values from them. (Acts as a graph output)
pub trait Sink: Entry + Send + Sync {
    /// Gets the values of the sink
    fn consume(&mut self) -> Block;
    /// Drop buffered frames without returning an allocated block.
    fn discard(&mut self) {
        let _ = self.consume();
    }
    fn get_frames(&self) -> &[Frame];
    fn into_entry(self) -> Box<dyn Entry>;
    /// Update a named parameter, returning `false` when it is unsupported.
    fn set_parameter(&mut self, _name: &str, _value: f32) -> bool {
        false
    }
}
dyn_clone::clone_trait_object!(Sink);
