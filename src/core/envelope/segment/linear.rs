use std::fmt;

/// A simple linear segment for an envelope.
/// The segment interpolates linearly between the start and end values over the specified duration.
#[derive(Debug, Clone)]
pub struct LinearSegment {
    from: f32,
    to: f32,
    duration: f32,
}

impl fmt::Display for LinearSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LinearSegment from {} to {} over {}s",
            self.from, self.to, self.duration
        )
    }
}

impl super::Segment for LinearSegment {
    fn at(&self, time: f32) -> f32 {
        if time <= 0.0 {
            return self.from;
        } else if time >= 1.0 {
            return self.to;
        }

        self.from + time * (self.to - self.from)
    }

    fn get_duration(&self) -> f32 {
        self.duration
    }
}

impl LinearSegment {
    pub fn new(from: f32, to: f32, duration: f32) -> Self {
        Self { from, to, duration }
    }

    pub fn default_attack() -> Self {
        Self {
            from: 0.0,
            to: 1.0,
            duration: 0.1,
        }
    }

    pub fn default_decay() -> Self {
        Self {
            from: 1.0,
            to: 0.8,
            duration: 0.1,
        }
    }

    pub fn default_release() -> Self {
        Self {
            from: 0.8,
            to: 0.0,
            duration: 0.2,
        }
    }
}