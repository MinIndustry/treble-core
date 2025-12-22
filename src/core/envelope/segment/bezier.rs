use std::fmt;

#[derive(Debug, Clone)]

pub struct BezierSegment {
    from: f32, // Time, Amplitude
    to: f32,
    duration: f32,
    control: (f32, f32),
}


impl fmt::Display for BezierSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} to {:?} - {:?}", self.from, self.to, self.control)
    }
}

impl super::Segment for BezierSegment {
        /// Returns the envelope value at the given point in absolute time
    /// If the given timestamp is before the segment, the returned
    /// value will be the `from` value of the segment. If the timestamp
    /// is after the segment, the returned value will be the `to` value
    /// of the segment
    fn at(&self, time: f32) -> f32 {
        (1.0 - time) * ((1.0 - time) * self.from + time * self.control.1)
            + time * ((1.0 - time) * self.control.1 + time * self.to)
    }

    fn get_duration(&self) -> f32 {
        self.duration
    }
}