use crate::core::envelope::prelude::ConstantSegment;
use crate::core::envelope::{prelude::Segment, Envelope};
use crate::core::generator::prelude::*;

#[test]
pub fn test_constant_segment_boundaries() {
    let segment: Box<dyn Segment> = Box::new(ConstantSegment::new(1.0, None));

    assert_eq!(segment.get_duration(), f32::INFINITY, "A segment with no duration is supposed infinite");
}

#[test]
pub fn test_constant_envelope_values() {
    let segment_value = 1.0;

    let segment: Box<dyn Envelope> = Box::new(ConstantSegment::new(segment_value, None));

    assert_eq!(segment.at(0.0, 0.0), segment_value, "A constant segment with value {segment_value} should always return this value");
    assert_eq!(segment.at(0.5, 0.0), segment_value, "A constant segment with value {segment_value} should always return this value");
    assert_eq!(segment.at(1.0, 0.0), segment_value, "A constant segment with value {segment_value} should always return this value");
}

#[test]
pub fn test_constant_generator() {
    let mut generator = builder::ToneGeneratorBuilder::new()
        .waveform(Waveform::Blank)
        .amplitude_envelope(Box::new(ConstantSegment::new(1.0, None)))
        .build();

    generator.start();

    assert_eq!(generator.tick(1.0 / 44100.0), 1.0, "Constant generator should always output its amplitude envelope's value");
}