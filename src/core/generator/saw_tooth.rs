use super::ToneGenerator;

#[derive(Debug)]
/// A generator that produces a sawtooth wave following the formula:
/// `y = A * (2 * ((t / T) - floor((1 / 2) + (t / T))))`
/// where A is the amplitude, T is the period, and t is the current time.
pub struct SawTooth {
    period: f32,
    amplitude: f32,
    timer: f32,
}

impl SawTooth {
    /// Generates a new sawtooth wave generator with the given frequency and amplitude.
    pub fn new(frequency: f32, amplitude: f32) -> Self {
        Self {
            period: 1.0 / frequency,
            amplitude,
            timer: 0.0,
        }
    }
}

impl ToneGenerator for SawTooth {
    fn tick(&mut self, elapsed_time: f32) -> f32 {
        self.timer += elapsed_time;
        self.amplitude
            * (2.0
                * ((self.timer / self.period) - ((1.0 / 2.0) + (self.timer / self.period)).floor()))
    }
}
