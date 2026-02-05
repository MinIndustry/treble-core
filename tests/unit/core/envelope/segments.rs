//! Envelope Segment Tests
//! Tests for individual envelope segments (constant, linear, bezier, function)

use rustic::core::envelope::prelude::ConstantSegment;
use rustic::core::envelope::{prelude::Segment, Envelope};
use rustic::core::generator::prelude::*;

#[test]
pub fn test_constant_segment_boundaries() {
    let segment: Box<dyn Segment> = Box::new(ConstantSegment::new(1.0, None));

    assert_eq!(
        segment.get_duration(),
        f32::INFINITY,
        "A segment with no duration is supposed infinite"
    );
}

#[test]
pub fn test_constant_envelope_values() {
    let segment_value = 1.0;

    let segment: Box<dyn Envelope> = Box::new(ConstantSegment::new(segment_value, None));

    assert_eq!(
        segment.at(0.0, 0.0),
        segment_value,
        "A constant segment with value {segment_value} should always return this value"
    );
    assert_eq!(
        segment.at(0.5, 0.0),
        segment_value,
        "A constant segment with value {segment_value} should always return this value"
    );
    assert_eq!(
        segment.at(1.0, 0.0),
        segment_value,
        "A constant segment with value {segment_value} should always return this value"
    );
}

#[test]
pub fn test_constant_generator() {
    let mut generator = builder::ToneGeneratorBuilder::new()
        .waveform(Waveform::Blank)
        .amplitude_envelope(Box::new(ConstantSegment::new(1.0, None)))
        .build();

    generator.start();

    assert_eq!(
        generator.tick(1.0 / 44100.0),
        1.0,
        "Constant generator should always output its amplitude envelope's value"
    );
}

#[cfg(test)]
mod linear_segment_tests {
    use rustic::core::envelope::prelude::{LinearSegment, Segment};

    // - Test linear interpolation between start and end values
    #[test]
    fn test_linear_segment_interp() {
        let segment = LinearSegment::new(0.0, 1.0, 1.0);
        let segment_2 = LinearSegment::new(1.0, 0.0, 0.16);

        assert_eq!(
            segment.at(0.0),
            0.0,
            "Start of the linear segment should be it minimum value"
        );
        assert_eq!(
            segment.at(1.0),
            1.0,
            "End of the linear segment should be its maximum value"
        );
        assert!(
            {
                let middle_segment = segment.at(0.5);

                middle_segment > 0.49 && middle_segment < 0.51
            },
            "Middle of the linear segment should be around 0.5"
        );

        assert!(
            segment_2.at(0.5) > 0.0,
            "Middle of linear segment {} can't be below 0.0 (reality: {})",
            segment_2,
            segment_2.at(0.5)
        );
    }

    // - Test segment duration
    #[test]
    fn test_linear_segment_duration_mapping() {
        let segment = LinearSegment::new(0.0, 1.0, 1.0);

        let mapped_duration = segment.map_time(0.0, 0.5);
        assert_eq!(
            mapped_duration, 0.5,
            "The mapped duration for the linear segment should be 0.5 (reality: {})",
            mapped_duration
        );

        let mapped_duration = segment.map_time(0.5, 1.0);
        assert_eq!(mapped_duration, 0.5, "The mapped duration for the linear segment with offset 0.5 should be 0.5 (reality: {})", mapped_duration);
    }

    // - Test boundary conditions (t=0, t=duration)
    #[test]
    fn test_linear_segment_boundaries() {
        let segment = LinearSegment::new(0.0, 1.0, 1.0);

        assert_eq!(
            segment.at(1.1),
            1.0,
            "Overflow on the segment value should return its maximum value"
        );
        assert_eq!(
            segment.at(-0.1),
            0.0,
            "Underflow on the segment value should return its minimum value"
        );
    }
}

#[cfg(test)]
mod bezier_segment_tests {
    // TODO: Add tests for BezierSegment
    // - Test bezier curve interpolation
    // - Test control point influence
    // - Test segment smoothness
}

#[cfg(test)]
mod function_segment_tests {
    // TODO: Add tests for FunctionSegment
    // - Test custom function evaluation
    // - Test function segment composition
}
