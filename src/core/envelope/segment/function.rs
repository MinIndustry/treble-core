use core::fmt;

#[derive(Clone)]
/// A segment of an envelope defined by a custom function.
pub struct FunctionSegment {
    function: fn(f32) -> f32,
    duration: Option<f32>,
}

impl fmt::Debug for FunctionSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FunctionSegment {{ duration: {:?} }}", self.duration)
    }
}

impl fmt::Display for FunctionSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FunctionSegment with duration {:?}", self.duration)
    }
}

impl super::Segment for FunctionSegment {
    fn at(&self, time: f32) -> f32 {
        (self.function)(time)
    }

    fn get_duration(&self) -> f32 {
        // Duration is not defined for a function segment; return a default value.
        self.duration.unwrap_or(f32::INFINITY)
    }
}

impl super::SustainSegment for FunctionSegment {}