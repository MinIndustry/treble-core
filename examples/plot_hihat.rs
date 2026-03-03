use simplelog::*;
use std::fs::File;
use std::{thread, time};

use rustic::instruments::prelude::HiHat;
use rustic::prelude::{App, AudioCommand, Command};
use rustic::{NOTES, Note};

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

    let mut app = App::init();

    let snare = HiHat::new().unwrap();

    app.instruments.push(Box::new(snare));

    log::info!("Starting rustic app");
    let event_rx = match app.start() {
        Ok(er) => er,
        Err(e) => {
            log::error!("Unable to start rustic app: {e:?}");
            return;
        }
    };

    std::thread::spawn(move || {
        log::info!("Event thread started");
        while let Ok(event) = event_rx.recv() {
            log::info!("Received event: {event:?}");
        }
    });

    let mut values: Vec<f32> = vec![];
    // let mut complete_value_list: Vec<f32> = vec![];

    values.clear();

    log::info!("Starting the hihat");
    let _ = app.send(Command::Audio(AudioCommand::NoteStart {
        instrument_idx: 0,
        note: Note::new(NOTES::A, 4),
        velocity: 1.0,
    }));

    // Play 10 seconds
    thread::sleep(time::Duration::from_secs(10));

    log::info!("Stopping hihat");
    let _ = app.send(Command::Audio(AudioCommand::NoteStop {
        instrument_idx: 0,
        note: Note::new(NOTES::A, 4),
    }));

    // Sleep two more seconds
    thread::sleep(time::Duration::from_secs(2));

    let _ = app.stop();

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
            "hihat.png",
        ) {
            log::error!("Error: {}", e.to_string());
        }
    }
}
