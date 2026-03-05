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
    /// Evaluates the bezier segment at normalized time `s` ∈ [0, 1].
    ///
    /// The control point `(cx, cy)` is in normalized space: `cx` ∈ [0, 1] is the
    /// *horizontal* position of the control point along the segment, and `cy` is its
    /// amplitude.  When `cx = 0.5` (the midpoint default) the mapping degenerates to
    /// a simple quadratic bezier in the amplitude axis only.
    ///
    /// For `cx ≠ 0.5` the parametric t is found by inverting the quadratic bezier
    /// x-component `X(t) = t²(1-2cx) + 2cx·t = s`, which yields:
    ///   `t = (−cx + √(cx² + s·(1−2cx))) / (1−2cx)`
    /// The discriminant is always ≥ 0 for `cx, s ∈ [0, 1]`.
    fn at(&self, time: f32) -> f32 {
        let s = time.clamp(0.0, 1.0);
        let cx = self.control.0.clamp(0.0, 1.0);
        let cy = self.control.1;

        let t = if (cx - 0.5).abs() < 1e-6 {
            s // linear parametric mapping when cx ≈ 0.5
        } else {
            let a = 1.0 - 2.0 * cx;
            let disc = (cx * cx + s * a).max(0.0).sqrt();
            ((-cx + disc) / a).clamp(0.0, 1.0)
        };

        (1.0 - t).powi(2) * self.from + 2.0 * (1.0 - t) * t * cy + t.powi(2) * self.to
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
