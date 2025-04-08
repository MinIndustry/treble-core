use super::ToneGenerator;
use rand;

#[derive(Debug)]
/// A generator that produces white noise following the formula:
/// `y = A * (rand::random::<f32>() * 2.0 - 1.0)`
/// where A is the amplitude.
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
