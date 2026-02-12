use crate::core::graph::{Entry, Sink};

/// A simple audio sink that stores incoming audio samples. (Allowing
/// other parts of the code to pull its values)
#[derive(Clone, Debug, Default)]
pub struct SimpleSink {
    values: Vec<f32>,
}

impl SimpleSink {
    pub fn new() -> Self {
        Self::default()
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

    fn into_entry(self) -> Box<dyn Entry> {
        Box::new(self)
    }
}
