use core::time;
use log::error;
use simplelog::*;
use std::fs::File;
use std::thread;

use rustic::instruments::prelude::{HiHat, Kick, Snare};
use rustic::prelude::{App, AudioCommand, Command};
use rustic::{NOTES, Note};

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

    log::info!("Starting engine");
    let mut app = App::init();

    let hihat = match HiHat::new() {
        Ok(h) => h,
        Err(e) => {
            error!("Unable to create hihat: {}", e);
            return;
        }
    };
    let kick = Kick::new();
    let snare = Snare::new();

    app.instruments.push(Box::new(hihat));
    app.instruments.push(Box::new(kick));
    app.instruments.push(Box::new(snare));

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

    // Wait a bit for the audio to start
    thread::sleep(time::Duration::from_millis(500));
    let mut values: Vec<f32> = vec![];

    // let mut complete_value_list: Vec<f32> = vec![];
    for i in 0..40 {
        values.clear();

        if i % 4 == 1 {
            if let Err(e) = app.send(Command::Audio(AudioCommand::NoteStart {
                instrument_idx: 1,
                note: Note::new(NOTES::A, 4),
                velocity: 1.0,
            })) {
                log::error!("Unable to start note: {}", e);
            }
        } else if i % 4 == 3 {
            let _ = app.send(Command::Audio(AudioCommand::NoteStart {
                instrument_idx: 2,
                note: Note::new(NOTES::A, 4),
                velocity: 1.0,
            }));
        } else if i < 39 {
            let _ = app.send(Command::Audio(AudioCommand::NoteStart {
                instrument_idx: 0,
                note: Note::new(NOTES::A, 4),
                velocity: 1.0,
            }));
        }

        thread::sleep(time::Duration::from_millis(250));

        let _ = app.send(Command::Audio(AudioCommand::NoteStop {
            instrument_idx: 0,
            note: Note::new(NOTES::A, 4),
        }));
        let _ = app.send(Command::Audio(AudioCommand::NoteStop {
            instrument_idx: 1,
            note: Note::new(NOTES::A, 4),
        }));
        let _ = app.send(Command::Audio(AudioCommand::NoteStop {
            instrument_idx: 2,
            note: Note::new(NOTES::A, 4),
        }));
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

    thread::sleep(time::Duration::from_secs(3));
    let _ = app.stop();
}
