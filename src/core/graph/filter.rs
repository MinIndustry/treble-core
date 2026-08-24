use crate::core::Block;
use crate::core::graph::Entry;
use std::fmt;

/// A filter that can process data. Data should be pushed to the filter's input by either the preceding filter or a source.
pub trait Filter: Entry + fmt::Display + treble_meta::MetaFilter + Send + Sync {
    /// Applies the filter's transformation to the input
    /// Returns a Vector of Blocks. Each block correspond to output port `x` of the filter
    fn transform(&mut self) -> Vec<Block>;

    /// Returns true if the filter's execution can be postponed to the end of the execution cycle of the graph.
    /// A postponable element must be present in a cycle of the graph to avoid infinite looping.
    /// E.g. a delay filter can be postponed if it lies within a feedback loop.
    fn postponable(&self) -> bool {
        false
    }

    /// Called at the start of each rendered block with the engine frame.
    ///
    /// Time-anchored filters (LFOs such as auto-pan and tremolo) derive their
    /// phase from this instead of from instance age, so a hot-swapped
    /// replacement filter continues at exactly the phase its predecessor had —
    /// editing a parameter mid-performance must not restart the sweep.
    /// Stateless filters ignore it.
    fn on_transport(&mut self, _frame: u64) {}

    /// Enables downcasting from trait object to concrete type.
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

dyn_clone::clone_trait_object!(Filter);
