use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink};

use log::error;

use rustic::instruments::prelude::{HiHat, Kick, Snare};
use rustic::instruments::Instrument;
use rustic::prelude::App;
use rustic::Note;

fn main() {
    colog::init();

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
            kick.start_note(Note(rustic::core::tones::NOTES::A, 0), 0.0);
        } else if i % 4 == 3 {
            snare.start_note(Note(rustic::core::tones::NOTES::A, 0), 0.0);
        } else if i < 39 {
            hihat.start_note(Note(rustic::core::tones::NOTES::A, 0), 0.0);
        }

        for _ in 0..(app.config.system.sample_rate as usize / 4) {
            kick.tick();
            hihat.tick();
            snare.tick();

            let hihat_output = hihat.get_output();
            let full = hihat_output + kick.get_output() + snare.get_output();

            values.push(full);
            values.push(full);

            complete_value_list.push(full);
            complete_value_list.push(full);
        }

        kick.stop_note(Note(rustic::core::tones::NOTES::A, 0));
        hihat.stop_note(Note(rustic::core::tones::NOTES::A, 0));

        sink.append(SamplesBuffer::new(
            2 as u16,
            app.config.system.sample_rate as u32,
            values.iter().map(|n| *n).collect::<Vec<f32>>(),
        ));
    }

    #[cfg(feature = "plotting")]
    {
        use rustic::plotting::Plot;

        let left_ear = complete_value_list
            .iter()
            .enumerate()
            .filter_map(|(position, element)| {
                if position % 2 == 0 {
                    Some((
                        (position as f32 / 2.0) / app.config.system.sample_rate,
                        *element,
                    ))
                } else {
                    None
                }
            })
            .collect();

        let right_ear = complete_value_list
            .iter()
            .enumerate()
            .filter_map(|(position, element)| {
                if position % 2 == 1 {
                    Some((
                        (position as f32 / 2.0) / app.config.system.sample_rate,
                        *element,
                    ))
                } else {
                    None
                }
            })
            .collect();

        let mut plot = Plot::new("Simple Drum", (-0.1, 1.0), (-0.8, 0.8), "drum.png");
        plot.plot(left_ear, "left ear", (255, 0, 0));
        plot.plot(right_ear, "right_ear", (0, 255, 0));
        plot.render();
    }

    sink.sleep_until_end();
}
