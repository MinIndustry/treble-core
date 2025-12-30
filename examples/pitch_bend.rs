use rustic::core::envelope::prelude::{ADSREnvelopeBuilder, BezierSegment, LinearSegment};
use rustic::core::utils::Note;

#[cfg(feature = "plotting")]
use rustic::plotting::plot_data;

fn main() {
    let scale = 0.5; // Master volume
    let sample_rate = 44100; // Sample rate

    let envelope = ADSREnvelopeBuilder::new()
        .attack(Box::new(BezierSegment::new(0.0, scale * 1.0, 0.01, (0.0, 1.0))))
        .decay(Box::new(LinearSegment::new(1.0, scale * 0.8, 0.1)))
        .release(Box::new(BezierSegment::new(scale * 0.8, 0.0, 2.0, (0.0, 0.0))))
        .build();

    let pitch_bend = LinearSegment::new(1.0, 0.1, 2.0);

    // let mut score = Score::new("Pitch bending".to_string(), sample_rate);
    // score.add_note(
    //     Note::new(100.0, 0.0, 5.0)
    //         .with_generator(GENERATORS::SINE)
    //         .with_envelope(Box::from(envelope))
    //         .with_pitch_bend(&pitch_bend),
    // );

    // score.play();
}
