use rustic::core::envelope::prelude::{ADSREnvelopeBuilder, BezierSegment, LinearSegment};
use rustic::instruments::prelude::{KeyboardBuilder, PolyVoiceAllocator};

use rustic::score::score_builder::ScoreBuilder;

#[cfg(feature = "plotting")]
use rustic::plotting::plot_data;

fn main() {
    let scale = 0.1; // Master volume

    let envelope = ADSREnvelopeBuilder::new()
        .attack(Box::new(BezierSegment::new(
            0.0,
            scale * 1.0,
            0.1,
            (0.05, 0.0),
        )))
        .decay(Box::new(LinearSegment::new(1.0, scale * 0.8, 1.0)))
        .release(Box::new(BezierSegment::new(
            scale * 0.8,
            0.0,
            0.2,
            (0.5, 0.0),
        )))
        .build();

    let _score = ScoreBuilder::new()
        .name("Morrowind")
        .with_instrument(Box::from(
            KeyboardBuilder::new()
                .with_note_envelope(envelope)
                .with_allocator(PolyVoiceAllocator::DropOldest)
                .with_voices(8)
                .build(),
        ));

    // let notes = {
    //     let note_duration: f32 = 0.3;

    //     let notes = vec![
    //         Note::new(TONES_FREQ[NOTES::C as usize][5], 0.0, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::C as usize][2], 0.0, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(
    //             TONES_FREQ[NOTES::D as usize][5],
    //             0.0 + (0.33 / 2.0),
    //             note_duration,
    //         )
    //         .with_generator(GENERATORS::SINE)
    //         .with_envelope(Box::from(envelope.clone())),
    //         Note::new(
    //             TONES_FREQ[NOTES::D as usize][2],
    //             0.0 + (0.33 / 2.0),
    //             note_duration,
    //         )
    //         .with_generator(GENERATORS::SINE)
    //         .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::DS as usize][5], 0.33, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::DS as usize][2], 0.33, 0.7)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::DS as usize][5], 1.0, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::DS as usize][3], 1.0, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(
    //             TONES_FREQ[NOTES::F as usize][5],
    //             1.0 + (0.33 / 2.0),
    //             note_duration,
    //         )
    //         .with_generator(GENERATORS::SINE)
    //         .with_envelope(Box::from(envelope.clone())),
    //         Note::new(
    //             TONES_FREQ[NOTES::F as usize][3],
    //             1.0 + (0.33 / 2.0),
    //             note_duration,
    //         )
    //         .with_generator(GENERATORS::SINE)
    //         .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::G as usize][5], 1.33, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::G as usize][3], 1.33, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::G as usize][5], 2.0, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::G as usize][3], 2.0, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(
    //             TONES_FREQ[NOTES::AS as usize][5],
    //             2.0 + (0.33 / 2.0),
    //             note_duration,
    //         )
    //         .with_generator(GENERATORS::SINE)
    //         .with_envelope(Box::from(envelope.clone())),
    //         Note::new(
    //             TONES_FREQ[NOTES::AS as usize][3],
    //             2.0 + (0.33 / 2.0),
    //             note_duration,
    //         )
    //         .with_generator(GENERATORS::SINE)
    //         .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::F as usize][5], 2.33, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::F as usize][3], 2.33, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::DS as usize][5], 3.0, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::DS as usize][3], 3.0, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(
    //             TONES_FREQ[NOTES::D as usize][5],
    //             3.0 + (0.33 / 2.0),
    //             note_duration,
    //         )
    //         .with_generator(GENERATORS::SINE)
    //         .with_envelope(Box::from(envelope.clone())),
    //         Note::new(
    //             TONES_FREQ[NOTES::D as usize][3],
    //             3.0 + (0.33 / 2.0),
    //             note_duration,
    //         )
    //         .with_generator(GENERATORS::SINE)
    //         .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::C as usize][5], 3.33, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::C as usize][3], 3.33, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::C as usize][5], 4.0, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::C as usize][3], 4.0, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(
    //             TONES_FREQ[NOTES::D as usize][5],
    //             4.0 + (0.33 / 2.0),
    //             note_duration,
    //         )
    //         .with_generator(GENERATORS::SINE)
    //         .with_envelope(Box::from(envelope.clone())),
    //         Note::new(
    //             TONES_FREQ[NOTES::D as usize][3],
    //             4.0 + (0.33 / 2.0),
    //             note_duration,
    //         )
    //         .with_generator(GENERATORS::SINE)
    //         .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::DS as usize][5], 4.33, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::DS as usize][3], 4.33, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::DS as usize][5], 5.0, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::DS as usize][3], 5.0, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(
    //             TONES_FREQ[NOTES::F as usize][5],
    //             5.0 + (0.33 / 2.0),
    //             note_duration,
    //         )
    //         .with_generator(GENERATORS::SINE)
    //         .with_envelope(Box::from(envelope.clone())),
    //         Note::new(
    //             TONES_FREQ[NOTES::F as usize][3],
    //             5.0 + (0.33 / 2.0),
    //             note_duration,
    //         )
    //         .with_generator(GENERATORS::SINE)
    //         .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::G as usize][5], 5.33, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::G as usize][3], 5.33, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::G as usize][5], 6.0, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::G as usize][3], 6.0, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(
    //             TONES_FREQ[NOTES::AS as usize][5],
    //             6.0 + (0.33 / 2.0),
    //             note_duration,
    //         )
    //         .with_generator(GENERATORS::SINE)
    //         .with_envelope(Box::from(envelope.clone())),
    //         Note::new(
    //             TONES_FREQ[NOTES::AS as usize][3],
    //             6.0 + (0.33 / 2.0),
    //             note_duration,
    //         )
    //         .with_generator(GENERATORS::SINE)
    //         .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::C as usize][6], 6.33, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::C as usize][4], 6.33, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::AS as usize][5], 7.0, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::AS as usize][3], 7.0, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(
    //             TONES_FREQ[NOTES::D as usize][6],
    //             7.0 + (0.33 / 2.0),
    //             note_duration,
    //         )
    //         .with_generator(GENERATORS::SINE)
    //         .with_envelope(Box::from(envelope.clone())),
    //         Note::new(
    //             TONES_FREQ[NOTES::D as usize][4],
    //             7.0 + (0.33 / 2.0),
    //             note_duration,
    //         )
    //         .with_generator(GENERATORS::SINE)
    //         .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::C as usize][6], 7.33, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::C as usize][4], 7.33, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::C as usize][6], 8.0, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::D as usize][6], 8.15, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::DS as usize][6], 8.3, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::D as usize][6], 8.6, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::C as usize][6], 8.9, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::AS as usize][5], 9.2, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::GS as usize][5], 9.5, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::G as usize][5], 9.8, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::F as usize][5], 10.1, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::DS as usize][5], 10.7, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::G as usize][5], 10.85, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::F as usize][5], 11.0, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::DS as usize][5], 11.7, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::D as usize][5], 11.85, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //         Note::new(TONES_FREQ[NOTES::C as usize][5], 12.0, note_duration)
    //             .with_generator(GENERATORS::SINE)
    //             .with_envelope(Box::from(envelope.clone())),
    //     ];

    //     notes
    // };

    // let mut score = Score::new("Morrowind".to_string(), sample_rate);

    // for note in notes.into_iter() {
    //     score.add_note(note);
    // }

    // score.play();
}
