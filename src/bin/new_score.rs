use log::error;
use std::path::PathBuf;

use rustic::instruments::prelude::Snare;
use rustic::prelude::notes;
use rustic::prelude::score::{Score, TimeSignature};
use rustic::prelude::App;

fn main() {
    App::init();

    let snare = Snare::new();

    let mut score = Score::new("Test score", TimeSignature::C, 120, 1, 20);
    let snare_staff = score.add_instrument(Box::from(snare));

    if let Err(e) = score.add_note(
        snare_staff,
        notes::Note::new(
            notes::NoteDuration::Crotchet,
            notes::DurationModifier::None,
            notes::NoteName::A,
            notes::NoteModifier::None,
            0,
            false,
        )
        .unwrap(),
    ) {
        error!("Unable to add note: {}", e.to_string());
    }

    if let Err(e) = score.save(&PathBuf::from("score.toml")) {
        error!("Unable to save score: {}", e.to_string());
    }
}
