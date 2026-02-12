use crate::core::graph::{Entry, Sink};

/// A sink that writes audio samples to the cpal ring buffer for playback
#[derive(Clone, Debug)]
pub struct AudioOutputSink {
    values: Vec<f32>,
}

impl Default for AudioOutputSink {
    fn default() -> Self {
        Self { values: Vec::new() }
    }
}

impl AudioOutputSink {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Entry for AudioOutputSink {
    fn push(&mut self, value: f32, _port: usize) {
        self.values.push(value);
    }
}

impl Sink for AudioOutputSink {
    fn consume(&mut self, amount: usize) -> Vec<f32> {
        let amount = std::cmp::min(amount, self.values.len());
        self.values.drain(0..amount).collect()
    }

    fn get_values(&self) -> Vec<f32> {
        self.values.clone()
    }

    fn into_entry(self) -> Box<dyn Entry> {
        Box::new(self)
    }
}
