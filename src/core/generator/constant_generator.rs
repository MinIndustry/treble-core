use super::ToneGenerator;

#[derive(Debug)]
/// A generator that does nothing. Can be used as a placeholder
/// or for testing purposes.
pub struct ConstantGenerator {
    value: f32,
}

impl ConstantGenerator {
    pub fn new(value: f32) -> Self {
        Self { value }
    }
}

impl ToneGenerator for ConstantGenerator {
    fn tick(&mut self, _elapsed_time: f32) -> f32 {
        self.value
    }
}
