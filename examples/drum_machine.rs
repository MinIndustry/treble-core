use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink};

use log::error;
use simplelog::*;
use std::fs::File;

use rustic::Note;
use rustic::instruments::Instrument;
use rustic::instruments::prelude::{HiHat, Kick, Snare};
use rustic::prelude::App;

#[cfg(feature = "plotting")]
use rustic::plotting::plot_data;

fn main() {
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Warn,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Warn,
            Config::default(),
            File::create("app.log").unwrap(),
        ),
    ])
    .unwrap();

    let app = App::init();

    let mut hihat = match HiHat::new() {
        Ok(h) => h,
        Err(e) => {
            error!("Unable to create hihat: {}", e);
            return;
        }
    };

    let mut kick = Kick::new();
    let mut snare = Snare::new();

    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    let mut values = vec![];
    let mut complete_value_list = vec![];
    for i in 0..40 {
        values.clear();

        if i % 4 == 1 {
            kick.start_note(Note(rustic::core::utils::tones::NOTES::A, 0), 0.0);
        } else if i % 4 == 3 {
            snare.start_note(Note(rustic::core::utils::tones::NOTES::A, 0), 0.0);
        } else if i < 39 {
            hihat.start_note(Note(rustic::core::utils::tones::NOTES::A, 0), 0.0);
        }

        for _ in 0..(app.config.system.sample_rate as usize / 4) {
            kick.tick();
            hihat.tick();
            snare.tick();

            let hihat_output = hihat.get_output();
            let full = hihat_output + kick.get_output() + snare.get_output();

            values.push(full); // Left
            values.push(full); // Right

            complete_value_list.push(full);
            complete_value_list.push(full);
        }

        kick.stop_note(Note(rustic::core::utils::tones::NOTES::A, 0));
        hihat.stop_note(Note(rustic::core::utils::tones::NOTES::A, 0));
        snare.stop_note(Note(rustic::core::utils::tones::NOTES::A, 0));

        sink.append(SamplesBuffer::new(
            2_u16,
            app.config.system.sample_rate,
            values.to_vec(),
        ));
    }

    #[cfg(feature = "plotting")]
    {
        let left_ear: Vec<(f32, f32)> = complete_value_list
            .iter()
            .enumerate()
            .map(|(position, element)| {
                (
                    (position as f32 / 2.0) / app.config.system.sample_rate as f32,
                    *element,
                )
            })
            .collect();

        if let Err(e) = plot_data(
            left_ear,
            "Drum machine Waveform",
            (-0.1, 1.1),
            (-1.1, 1.1),
            "drum_machine.png",
        ) {
            log::error!("Error: {}", e.to_string());
        }
    }

    sink.sleep_until_end();
}
