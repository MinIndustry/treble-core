use std::sync::Arc;

use crate::core::graph::{Entry, Sink};
use crate::core::{Block, Frame};

/// A sink that writes audio samples to the cpal ring buffer for playback
#[derive(Clone, Default, Debug)]
pub struct AudioOutputSink {
    values: Vec<Frame>,
}

impl AudioOutputSink {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Entry for AudioOutputSink {
    fn push(&mut self, block: Arc<Block>, _port: usize) {
        // Get back to owned data
        self.values.extend(block.iter().map(|f| [f[0], f[1]]));
    }
}

impl Sink for AudioOutputSink {
    fn consume(&mut self) -> Block {
        // let amount = std::cmp::min(n_frames, self.values.len());
        self.values.drain(..).collect()
    }

    fn get_frames(&self) -> &[Frame] {
        &self.values
    }

    fn into_entry(self) -> Box<dyn Entry> {
        Box::new(self)
    }
}
