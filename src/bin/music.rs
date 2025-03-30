use rustic::core::envelope::prelude::{ADSREnvelope, Segment};
use rustic::core::generator::GENERATORS;
use rustic::core::tones::{NOTES, TONES_FREQ};
use rustic::core::{note::Note, score::Score};

#[cfg(feature = "plotting")]
use rustic::plotting::plot_data;

fn main() {
    let scale = 0.1; // Master volume
    let sample_rate = 44100; // Sample rate

    let envelope = {
        let mut env = ADSREnvelope::new();
        env.set_attack(0.2, scale * 1.0, Some((0.2, 0.0)));
        env.set_decay(0.05, scale * 0.8, None);
        env.set_release(2.0, scale * 0.0, Some((0.5, 0.0)));
        env
    };

    let drum_envelope = {
        let mut env = ADSREnvelope::new();
        env.set_attack(0.04, scale * 1.0, Some((0.0, 1.0)));
        env.set_decay(0.0, scale * 1.0, None);
        env.set_release(0.26, scale * 0.0, Some((0.0, 0.0)));
        env
    };

    let pitch_bend = Segment::new(1.0, 0.0, 0.3, 0.0, Some((2.0, 0.2)));

    let notes = {
        let mut notes = vec![
            Note::new(TONES_FREQ[NOTES::C as usize][5], 0.0, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::C as usize][2], 0.0, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::D as usize][5], 0.0 + (0.33 / 2.0), 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::D as usize][2], 0.0 + (0.33 / 2.0), 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::DS as usize][5], 0.33, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::DS as usize][2], 0.33, 0.7)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::DS as usize][5], 1.0, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::DS as usize][3], 1.0, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::F as usize][5], 1.0 + (0.33 / 2.0), 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::F as usize][3], 1.0 + (0.33 / 2.0), 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::G as usize][5], 1.33, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::G as usize][3], 1.33, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::G as usize][5], 2.0, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::G as usize][3], 2.0, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::AS as usize][5], 2.0 + (0.33 / 2.0), 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::AS as usize][3], 2.0 + (0.33 / 2.0), 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::F as usize][5], 2.33, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::F as usize][3], 2.33, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::DS as usize][5], 3.0, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::DS as usize][3], 3.0, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::D as usize][5], 3.0 + (0.33 / 2.0), 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::D as usize][3], 3.0 + (0.33 / 2.0), 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::C as usize][5], 3.33, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::C as usize][3], 3.33, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::C as usize][5], 4.0, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::C as usize][3], 4.0, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::D as usize][5], 4.0 + (0.33 / 2.0), 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::D as usize][3], 4.0 + (0.33 / 2.0), 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::DS as usize][5], 4.33, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::DS as usize][3], 4.33, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::DS as usize][5], 5.0, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::DS as usize][3], 5.0, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::F as usize][5], 5.0 + (0.33 / 2.0), 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::F as usize][3], 5.0 + (0.33 / 2.0), 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::G as usize][5], 5.33, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::G as usize][3], 5.33, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::G as usize][5], 6.0, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::G as usize][3], 6.0, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::AS as usize][5], 6.0 + (0.33 / 2.0), 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::AS as usize][3], 6.0 + (0.33 / 2.0), 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::C as usize][6], 6.33, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::C as usize][4], 6.33, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::AS as usize][5], 7.0, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::AS as usize][3], 7.0, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::D as usize][6], 7.0 + (0.33 / 2.0), 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::D as usize][4], 7.0 + (0.33 / 2.0), 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::C as usize][6], 7.33, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::C as usize][4], 7.33, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::C as usize][6], 8.0, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::D as usize][6], 8.15, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::DS as usize][6], 8.3, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::D as usize][6], 8.6, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::C as usize][6], 8.9, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::AS as usize][5], 9.2, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::GS as usize][5], 9.5, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::G as usize][5], 9.8, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::F as usize][5], 10.1, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::DS as usize][5], 10.7, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::G as usize][5], 10.85, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::F as usize][5], 11.0, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::DS as usize][5], 11.7, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::D as usize][5], 11.85, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
            Note::new(TONES_FREQ[NOTES::C as usize][5], 12.0, 0.2)
                .with_generator(GENERATORS::SINE)
                .with_envelope(&envelope),
        ];

        for i in 0..12 {
            notes.push(
                Note::new(TONES_FREQ[NOTES::A as usize][3], i as f32, 0.3)
                    .with_generator(GENERATORS::SINE)
                    .with_pitch_bend(&pitch_bend)
                    .with_envelope(&drum_envelope),
            );
        }

        notes
    };

    let mut score = Score::new("Morrowind".to_string(), sample_rate);

    for note in notes.into_iter() {
        score.add_note(note);
    }

    score.play();
}
