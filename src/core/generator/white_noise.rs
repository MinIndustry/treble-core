use super::ToneGenerator;
use rand;

#[derive(Debug)]
pub struct WhiteNoise {
    amplitude: f32,
}

impl WhiteNoise {
    pub fn new(amplitude: f32) -> Self {
        Self { amplitude }
    }
}

impl ToneGenerator for WhiteNoise {
    fn tick(&mut self, _elapsed_time: f32) -> f32 {
        self.amplitude * (rand::random::<f32>() * 2.0 - 1.0)
    }
}
