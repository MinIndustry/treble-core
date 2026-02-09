use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink};

use simplelog::*;
use std::fs::File;

use rustic::Note;
use rustic::instruments::Instrument;
use rustic::instruments::prelude::Kick;
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

    let mut kick = Kick::new();

    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    let mut values = vec![];
    let mut complete_value_list = vec![];
    for i in 0..20 {
        values.clear();

        if i % 4 == 1 {
            println!("Starting kick at position {}", i);
            kick.start_note(Note(rustic::core::utils::tones::NOTES::A, 0), 0.0);
        }

        for _ in 0..(app.config.system.sample_rate as usize / 4) {
            kick.tick();

            let full = kick.get_output();

            values.push(full); // Left
            values.push(full); // Right

            complete_value_list.push(full);
            complete_value_list.push(full);
        }

        kick.stop_note(Note(rustic::core::utils::tones::NOTES::A, 0));

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
            "Multi Kick Waveform",
            (-0.1, 1.1),
            (-1.1, 1.1),
            "multi_kick.png",
        ) {
            log::error!("Error: {}", e.to_string());
        }
    }

    sink.sleep_until_end();
}
