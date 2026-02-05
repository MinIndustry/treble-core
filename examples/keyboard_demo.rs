use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink};
use rustic::core::envelope::prelude::{ADSREnvelopeBuilder, BezierSegment};
use std::collections::HashMap;
use std::thread;

use simplelog::*;
use std::fs::File;

use log::{error, info, warn};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[cfg(feature = "linux")]
use evdev::EventType;

use rustic::core::utils::tones::NOTES;
use rustic::inputs::keyboard::*;
use rustic::instruments::{prelude::Keyboard, Instrument};
use rustic::Note;

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

    let sample_rate = 44100.0; // 44100 Hz

    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    let keyboard: Keyboard = Keyboard::new(
        4,
        rustic::instruments::prelude::PolyVoiceAllocator::DropOldest,
        ADSREnvelopeBuilder::new()
            .attack(Box::new(BezierSegment::new(0.0, 1.0, 0.2, (0.2, 0.0))))
            .decay(Box::new(BezierSegment::new(1.0, 0.8, 0.3, (0.3, 1.0))))
            .release(Box::new(BezierSegment::new(0.8, 0.0, 1.5, (0.0, 0.0))))
            .build(),
    );

    let keyboard = Arc::new(Mutex::new(keyboard));
    let _keyboard_2 = keyboard.clone();

    thread::spawn(move || {
        let _input_device = if let Some(device) = find_keyboard() {
            device
        } else {
            error!("No keyboard found for playing");
            return;
        };

        let _mapping = HashMap::from([
            (16, Note(NOTES::C, 4)),
            (17, Note(NOTES::CS, 4)),
            (18, Note(NOTES::D, 4)),
            (19, Note(NOTES::DS, 4)),
            (20, Note(NOTES::E, 4)),
            (21, Note(NOTES::F, 4)),
            (22, Note(NOTES::FS, 4)),
            (23, Note(NOTES::G, 4)),
            (24, Note(NOTES::GS, 4)),
            (25, Note(NOTES::A, 4)),
            (26, Note(NOTES::AS, 4)),
            (27, Note(NOTES::B, 4)),
        ]);

        #[cfg(feature = "linux")]
        loop {
            match input_device.fetch_events() {
                Ok(events) => {
                    for event in events {
                        if event.event_type() == EventType::KEY && event.value() != 2 {
                            if mapping.contains_key(&event.code()) {
                                let note = *mapping.get(&event.code()).unwrap();
                                match keyboard_2.lock() {
                                    Ok(mut keyboard) => {
                                        if event.value() == 1 {
                                            info!("Starting note: {:?}", note);
                                            keyboard.start_note(note, 1.0);
                                        } else {
                                            info!("Stopping note: {:?}", note);
                                            keyboard.stop_note(note);
                                        }
                                    }
                                    Err(_) => {
                                        warn!("Failed to borrow keyboard");
                                        continue;
                                    }
                                };
                            } else {
                                info!(
                                    "Event:\t{:?}\t{:?}\t{:?}",
                                    event.event_type(),
                                    event.code(),
                                    event.value()
                                );
                            }
                        }
                    }
                }
                Err(_) => {
                    info!("Error fetching events");
                }
            }
            thread::sleep(time::Duration::from_millis(10));
        }
    });

    let mut values = vec![];
    loop {
        values.clear();
        let start_time = Instant::now();
        for _ in 0..(0.05 * sample_rate) as usize {
            match keyboard.lock() {
                Ok(mut keyboard) => {
                    keyboard.tick();
                    values.push(keyboard.get_output());
                }
                Err(_) => {
                    warn!("Failed to borrow keyboard");
                    continue;
                }
            };
        }

        values.iter().for_each(|v| println!("{}", v));
        sink.append(SamplesBuffer::new(
            1 as u16,
            sample_rate as u32,
            values.clone(),
        ));
        info!(
            "Added 50ms in {}ms",
            Instant::now().duration_since(start_time).as_millis()
        );
    }
}
