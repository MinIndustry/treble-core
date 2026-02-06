use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink};

use simplelog::*;
use std::fs::File;

use rustic::Note;
use rustic::instruments::Instrument;
use rustic::instruments::prelude::HiHat;
use rustic::prelude::App;

#[cfg(feature = "plotting")]
use rustic::plotting::plot_data;

fn main() {
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Trace,
            Config::default(),
            File::create("app.log").unwrap(),
        ),
    ])
    .unwrap();

    let app = App::init();

    let mut snare = HiHat::new().unwrap();

    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    let mut values = vec![];
    let mut complete_value_list = vec![];

    values.clear();

    snare.start_note(Note(rustic::core::utils::tones::NOTES::A, 0), 0.0);

    for _ in 0..(app.config.system.sample_rate as usize) {
        snare.tick();

        let full = snare.get_output();

        values.push(full);

        complete_value_list.push(full);
    }

    snare.stop_note(Note(rustic::core::utils::tones::NOTES::A, 0));

    sink.append(SamplesBuffer::new(
        1_u16,
        app.config.system.sample_rate,
        values.to_vec(),
    ));

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
            "Snare Waveform",
            (-0.1, 1.1),
            (-1.1, 1.1),
            "snare.png",
        ) {
            log::error!("Error: {}", e.to_string());
        }
    }

    sink.sleep_until_end();
}
