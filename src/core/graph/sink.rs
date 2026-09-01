use crate::core::graph::Entry;
use crate::core::{Block, Frame};

/// Measurements captured while a sink produced its most recent block.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SinkTelemetry {
    /// Loudest absolute sample after master gain but before limiting.
    pub pre_limiter_peak: f32,
    /// Loudest absolute sample emitted by the sink.
    pub post_limiter_peak: f32,
    /// Greatest gain reduction applied in the block, in positive decibels.
    pub max_gain_reduction_db: f32,
    /// Number of individual channel samples that required limiting.
    pub limited_samples: usize,
}

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
    /// Telemetry for the most recently consumed block, when supported.
    fn telemetry(&self) -> Option<SinkTelemetry> {
        None
    }
}
dyn_clone::clone_trait_object!(Sink);
