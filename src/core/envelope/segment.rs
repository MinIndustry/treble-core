use super::Shape;
use std::default::Default;
use std::fmt;

use log::warn;

#[derive(Debug, Clone)]
pub struct Segment {
    from: (f32, f32), // Time, Amplitude
    to: (f32, f32),
    control: Option<(f32, f32)>,
}

impl Default for Segment {
    fn default() -> Self {
        Self {
            from: (0.0, 1.0),
            to: (1.0, 1.0),
            control: None,
        }
    }
}

impl fmt::Display for Segment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} to {:?} - {:?}", self.from, self.to, self.control)
    }
}

impl Segment {
    pub fn new(
        from: f32,
        to: f32,
        duration: f32,
        offset: f32,
        control: Option<(f32, f32)>,
    ) -> Self {
        Self {
            from: (offset, from),
            to: (offset + duration, to),
            control: control.map(|(x, y)| (x + offset, y)),
        }
    }

    pub fn change_from(&mut self, new_from: f32) {
        self.from.1 = new_from;
    }

    /// Returns true if the given time is covered by this segment.
    pub fn covers(&self, time: f32) -> bool {
        self.from.0 <= time && time <= self.to.0
    }

    //. Returns the start time for the current segment
    pub fn start(&self) -> f32 {
        self.from.0
    }

    //. Returns the start time for the current segment
    pub fn start_value(&self) -> f32 {
        self.from.1
    }

    //. Returns the end time for the current segment
    pub fn end(&self) -> f32 {
        self.to.0
    }

    //. Returns the end time for the current segment
    pub fn end_value(&self) -> f32 {
        self.to.1
    }

    /// Returns the envelope value at the given point in absolute time
    /// If the given timestamp is before the segment, the returned
    /// value will be the `from` value of the segment. If the timestamp
    /// is after the segment, the returned value will be the `to` value
    /// of the segment
    pub fn at(&self, time: f32) -> f32 {
        let (x0, y0) = self.from;
        let (x1, y1) = self.to;

        // Normalise time over the segment
        let time = (time - x0) / (x1 - x0);

        if time < 0.0 {
            warn!("Asking for time before the segment starts");
            return y0;
        } else if time > 1.0 {
            warn!("Asking for time after the segment ends");
            return y1;
        }

        if let Some(control) = self.control {
            // Bezier
            (1.0 - time) * ((1.0 - time) * self.from.1 + time * control.1)
                + time * ((1.0 - time) * control.1 + time * self.to.1)
        } else {
            // Simple linear interp
            y0 - ((y0 - y1) * time)
        }
    }
}

impl Shape for Segment {
    fn get_at(&self, time: f32) -> f32 {
        self.at(time)
    }
}
