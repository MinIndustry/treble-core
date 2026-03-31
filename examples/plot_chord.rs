use simplelog::*;
use std::fs::File;
use std::{thread, time};

use rustic::audio::{AudioEvent, BackendEvent, EventFilter, StatusEvent};
use rustic::instruments::prelude::KeyboardBuilder;
use rustic::prelude::{App, AudioCommand, Command};
use rustic::{NOTES, Note};

#[cfg(feature = "plotting")]
use rustic::app::prelude::FSConfig;
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
            LevelFilter::Warn,
            Config::default(),
            File::create("app.log").unwrap(),
        ),
    ])
    .unwrap();

    let mut app = App::init().unwrap();

    let keyboard = KeyboardBuilder::new().with_voices(6).build();
    app.add_instrument(Box::new(keyboard));

    log::info!("Starting rustic app");
    let event_rx = match app.start(EventFilter::all()) {
        Ok(er) => er,
        Err(e) => {
            log::error!("Unable to start rustic app: {e:?}");
            return;
        }
    };

    let sample_rate = app.config.system.sample_rate;

    // Collect audio chunks from the render thread for plotting
    let capture_handle = std::thread::spawn(move || {
        let mut samples: Vec<f32> = vec![];
        while let Ok(event) = event_rx.recv() {
            match event {
                BackendEvent::Audio(AudioEvent::Chunk(chunk)) => {
                    // Chunks are stereo-interleaved (L, R, L, R, …); take left channel only
                    samples.extend(chunk.into_iter().step_by(2));
                }
                BackendEvent::Status(StatusEvent::AudioStopped) => break,
                _ => {}
            }
        }
        samples
    });

    log::info!("Starting C major chord (C4, E4, G4)");
    for &(note_name, octave) in &[(NOTES::C, 4), (NOTES::E, 4), (NOTES::G, 4)] {
        let _ = app.send(Command::Audio(AudioCommand::NoteStart {
            instrument_idx: 0,
            note: Note::new(note_name, octave),
            velocity: 1.0,
        }));
    }

    thread::sleep(time::Duration::from_secs(2));
    let _ = app.stop();

    let samples = capture_handle.join().unwrap_or_default();
    log::info!(
        "Captured {} mono samples ({:.2}s)",
        samples.len(),
        samples.len() as f32 / sample_rate as f32
    );

    #[cfg(feature = "plotting")]
    {
        let waveform: Vec<(f32, f32)> = samples
            .iter()
            .enumerate()
            .map(|(i, &s)| (i as f32 / sample_rate as f32, s))
            .collect();

        let output_path = FSConfig::debug_dir("examples", "chord.png")
            .unwrap_or_else(|_| std::path::PathBuf::from("chord.png"));

        if let Err(e) = plot_data(
            waveform,
            "C Major Chord Waveform",
            (-0.1, 2.3),
            (-1.1, 1.1),
            output_path.to_str().unwrap_or("chord.png"),
        ) {
            log::error!("Error: {}", e);
        }
    }
}
