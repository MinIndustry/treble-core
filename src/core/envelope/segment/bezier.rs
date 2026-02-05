use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Represents a Bezier Segment for an envelope. A bezier segment is defined by a starting point,
/// an ending point and a control point. Since segments times are normalized between 0.0 and 1.0,
/// the x axis values for the start and end points are, respectively, 0.0 and 1.0.
///
/// The control point will define the shape of the curve between the start and end points. A control point below the
/// line segment formed by the start and end point will cause the curve to bend downwards, similar to an exponential function.
/// Inversely, a control point above the line segment will cause the curve to bend upwards, similar to a logarithmic function.
/// A control point on the line segment will cause the curve to be linear.
pub struct BezierSegment {
    from: f32,
    to: f32,
    duration: f32,
    control: (f32, f32),
}

impl BezierSegment {
    pub fn new(from: f32, to: f32, duration: f32, control_point: (f32, f32)) -> Self {
        BezierSegment {
            from,
            to,
            duration,
            control: control_point,
        }
    }
}

impl fmt::Display for BezierSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} to {:?} - {:?}", self.from, self.to, self.control)
    }
}

#[typetag::serde]
impl super::Segment for BezierSegment {
    fn at(&self, time: f32) -> f32 {
        (1.0 - time) * ((1.0 - time) * self.from + time * self.control.1)
            + time * ((1.0 - time) * self.control.1 + time * self.to)
    }

    fn get_duration(&self) -> f32 {
        self.duration
    }
}

#[typetag::serde]
impl super::super::Envelope for BezierSegment {
    fn at(&self, time: f32, note_off: f32) -> f32 {
        if (note_off > 0.0 && time >= note_off) || time >= self.duration {
            return self.to;
        }
        super::Segment::at(self, (time - note_off) / super::Segment::get_duration(self))
    }

    fn completed(&self, time: f32, note_off: f32) -> bool {
        (time - note_off) >= super::Segment::get_duration(self)
    }
}
