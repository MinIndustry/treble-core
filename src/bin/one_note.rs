use rustic::core::envelope::prelude::ADSREnvelope;
use rustic::core::generator::GENERATORS;
use rustic::core::tones::{NOTES, TONES_FREQ};
use rustic::core::{note::Note, score::Score};

use rustic::prelude::App;

fn main() {
    let app = App::init();
    let scale = app.config.system.master_volume;
    let sample_rate = app.config.system.sample_rate;

    let envelope = {
        let mut env = ADSREnvelope::new();
        env.set_attack(0.1, scale * 1.0, Some((0.1, 0.0)));
        env.set_decay(0.4, scale * 0.2, Some((0.5, scale * 1.0)));
        env.set_release(2.0, scale * 0.0, Some((0.5, 0.0)));
        env
    };

    let notes = vec![
        Note::new(TONES_FREQ[NOTES::C as usize][3], 0.0, 5.0)
            .with_generator(GENERATORS::SINE)
            .with_envelope(&envelope),
        Note::new(TONES_FREQ[NOTES::E as usize][3], 0.0, 5.0)
            .with_generator(GENERATORS::SINE)
            .with_envelope(&envelope),
        Note::new(TONES_FREQ[NOTES::G as usize][3], 0.0, 5.0)
            .with_generator(GENERATORS::SINE)
            .with_envelope(&envelope),
    ];

    let mut score = Score::new("Long note".to_string(), sample_rate as i32);

    for note in notes.into_iter() {
        score.add_note(note);
    }

    score.play();
}
