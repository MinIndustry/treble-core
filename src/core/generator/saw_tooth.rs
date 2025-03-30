use super::ToneGenerator;

#[derive(Debug)]
pub struct SawTooth {
    period: f32,
    amplitude: f32,
    timer: f32,
}

impl SawTooth {
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
