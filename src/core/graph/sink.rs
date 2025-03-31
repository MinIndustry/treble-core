use crate::core::graph::{AudioGraphElement, Entry, Sink};

/// Low-pass filter using a first-order IIR filter
#[derive(Clone)]
pub struct SimpleSink {
    values: Vec<f32>,
    index: usize,
}

impl SimpleSink {
    pub fn new() -> Self {
        Self {
            values: Vec::new(),
            index: 0,
        }
    }
}

impl Entry for SimpleSink {
    fn push(&mut self, value: f32, _port: usize) {
        // trace!("{}", value);
        self.values.push(value);
    }
}

impl Sink for SimpleSink {
    fn consume(&mut self, amount: usize) -> Vec<f32> {
        let amount = std::cmp::min(amount, self.values.len());
        self.values.drain(0..amount).collect()
    }

    fn get_values(&self) -> Vec<f32> {
        self.values.clone()
    }

    fn as_entry(self) -> Box<dyn Entry> {
        Box::new(self)
    }
}

impl AudioGraphElement for SimpleSink {
    fn get_name(&self) -> &str {
        "Audio Sink"
    }

    fn get_index(&self) -> usize {
        self.index
    }

    fn set_index(&mut self, index: usize) {
        self.index = index;
    }
}

pub fn simple_sink() -> Box<dyn Sink> {
    Box::new(SimpleSink::new())
}
