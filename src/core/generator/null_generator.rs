use super::ToneGenerator;

#[derive(Debug)]
pub struct NullGenerator;

impl NullGenerator {
    pub fn new() -> Self {
        Self
    }
}

impl ToneGenerator for NullGenerator {
    fn tick(&mut self, _elapsed_time: f32) -> f32 {
        0.0
    }
}
