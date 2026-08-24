use std::sync::Arc;

use crate::core::graph::{Entry, Sink};
use crate::core::{Block, Frame};

/// A simple audio sink that stores incoming audio samples. (Allowing
/// other parts of the code to pull its values)
#[derive(Clone, Debug, Default)]
pub struct SimpleSink {
    values: Vec<Frame>,
}

impl SimpleSink {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Entry for SimpleSink {
    fn push(&mut self, block: Arc<Block>, _port: usize) {
        self.values.extend(block.iter().map(|f| [f[0], f[1]]));
    }
}

impl Sink for SimpleSink {
    fn consume(&mut self) -> Block {
        // Move the buffer out rather than draining into a fresh allocation:
        // this runs once per block per sink.
        std::mem::take(&mut self.values)
    }

    fn discard(&mut self) {
        self.values.clear();
    }

    fn get_frames(&self) -> &[Frame] {
        &self.values
    }

    fn into_entry(self) -> Box<dyn Entry> {
        Box::new(self)
    }
}
