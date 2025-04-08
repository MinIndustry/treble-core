use std::f32::consts::PI;

use super::ToneGenerator;

#[derive(Debug)]
/// A generator that produces a square wave following the formula:
/// `y = A * sign(sin(2 * PI * f * t))`
/// where A is the amplitude, f is the frequency, and t is the current time.
pub struct SquareWave {
    frequency: f32,
    amplitude: f32,
    timer: f32,
}

impl SquareWave {
    pub fn new(frequency: f32, amplitude: f32) -> Self {
        Self {
            frequency,
            amplitude,
            timer: 0.0,
        }
    }
}

impl ToneGenerator for SquareWave {
    fn tick(&mut self, elapsed_time: f32) -> f32 {
        self.timer += elapsed_time;
        (2.0 * PI * self.frequency * self.timer).sin().signum() * self.amplitude
    }
}
