use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use log::error;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// The `app` module contains the main application data structures and functions.
/// It provides CLI utilities for managing the application as well as filesystem
/// utilities for managing files and directories.
mod app;

/// The core module of rustic. Contains the envelopes, filters, generators and the
/// graph building utilities.
pub mod core;

/// Instruments are structures that implement the `Instrument` trait.
pub mod instruments;

#[cfg(feature = "meta")]
/// This module defines the metadata structures for the application.
/// It allows to store and retreive metadata about filters
pub mod meta;

/// The mod score contains all the building block for creating music
/// Sheets contain instruments layed out on a staff, divided into measures
/// Notes in the measures are structures that implement the `MeasureNote` trait.
/// This allows to build complex notes, chords, ...
pub mod score;

const APP_ID: (&str, &str, &str) = ("rustic", "minigrim0", "xyz");

/// Main prelude module that exports the most commonly used types from the crate
pub mod prelude {
    // App exports
    pub use super::app::{App, AppMode, Commands, RunMode};

    // Core exports - only expose the module, details accessed through it
    pub use super::core;

    // Score exports
    pub use super::score::{
        Chord, ChordModifier, DurationModifier, Measure, Note, NoteDuration, NoteModifier,
        NoteName, Score, Staff, StaffInstance, TimeSignature,
    };

    // Instruments exports
    pub use super::instruments::Instrument;
}

use crate::core::generator::{Bendable, Generator};
use core::tones::NOTES;

#[cfg(feature = "plotting")]
pub mod plotting;

#[cfg(test)]
pub mod tests;

pub trait KeyboardGenerator: Generator + Bendable + Send + Sync {}

/// A note with its octave
#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone)]
pub struct Note(pub NOTES, pub u8);

pub fn start_app(
    sender: Sender<prelude::Commands>,
    receiver: Receiver<prelude::Commands>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut rustic_app = prelude::App::new();
        rustic_app.set_mode(prelude::RunMode::Live);

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("no output device available");
        let mut supported_configs_range = device
            .supported_output_configs()
            .expect("error while querying configs");
        let supported_config = supported_configs_range
            .next()
            .expect("no supported config?!")
            .with_max_sample_rate();
        let mut config = supported_config.config();
        config.buffer_size = cpal::BufferSize::Fixed(64);
        rustic_app.config.system.sample_rate = config.sample_rate.0;
        let mut command_batch = Vec::with_capacity(16);

        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    println!("Processing audio");
                    // react to stream events and read or write stream data here.
                    command_batch.clear();
                    while let Ok(command) = receiver.try_recv() {
                        println!("received {command:?}");
                        command_batch.push(command);
                        if command_batch.len() >= 16 {
                            break;
                        } // Prevent excessive processing
                    }

                    // Process batch
                    for command in &command_batch {
                        rustic_app.on_event(command.clone());
                    }

                    for sample in data.iter_mut() {
                        let new_sample = rustic_app.live_tick();
                        *sample = new_sample;
                    }
                },
                move |err| {
                    // react to errors here.
                    error!("An error occured: {}", err.to_string());
                },
                None, // None=blocking, Some(Duration)=timeout
            )
            .unwrap();

        stream.play().unwrap();
        loop {}
    })
}
