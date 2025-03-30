use std::fmt;

use super::Envelope;

pub struct BezierEnvelope {
    from_point: (f32, f32),
    to_point: (f32, f32),
    control: (f32, f32),
}

impl BezierEnvelope {
    pub fn new(from: f32, to: f32, duration: f32, control: (f32, f32)) -> Self {
        Self {
            from_point: (0.0, from),
            to_point: (duration, to),
            control,
        }
    }
}

impl fmt::Display for BezierEnvelope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bezier: {:?} to {:?} - {:?}", self.from_point, self.to_point, self.control)
    }
}

impl Envelope for BezierEnvelope {
    fn at(&self, time: f32) -> f32 {
        let duration = self.to_point.0;
        if time < 0.0 {
            self.from_point.1
        } else if time > duration {
            self.to_point.1
        } else {
            let progress = time / duration;

            // Calculate the current value of the bezier curve
            (1.0 - progress).powi(2) * self.from_point.1
                + 2.0 * (1.0 - progress) * progress * self.control.1
                + progress.powi(2) * self.to_point.1
        }
    }
}
