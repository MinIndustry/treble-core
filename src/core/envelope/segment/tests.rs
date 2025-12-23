use super::*;

#[test]
pub fn test_constant_segment_boundaries() {
    let segment: Box<dyn Segment> = Box::new(ConstantSegment::new(1.0, None));

    assert_eq!(segment.get_duration(), f32::INFINITY, "A segment with no duration is supposed infinite");
}
