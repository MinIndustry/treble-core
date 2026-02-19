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
    fn push(&mut self, block: Block, _port: usize) {
        self.values.extend(block);
    }
}

impl Sink for SimpleSink {
    fn consume(&mut self) -> Block {
        // let amount = std::cmp::min(amount, self.values.len());
        self.values.drain(..).collect()
    }

    fn get_frames(&self) -> &[Frame] {
        &self.values
    }

    fn into_entry(self) -> Box<dyn Entry> {
        Box::new(self)
    }
}
