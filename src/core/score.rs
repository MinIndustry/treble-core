use std::collections::BinaryHeap;

use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink};

use log::info;

use super::note::Note;

#[cfg(feature = "plotting")]
use crate::core::envelope::Envelope;
#[cfg(feature = "plotting")]
use crate::plotting::Plot;
#[cfg(feature = "plotting")]
use log::error;

pub struct Score {
    notes: BinaryHeap<Note>,
    _playing: bool,
    current_sample: i32,
    sample_rate: i32,
    _name: String,
}

impl Score {
    pub fn new(name: String, sample_rate: i32) -> Self {
        Self {
            notes: BinaryHeap::new(),
            _playing: false,
            current_sample: 0,
            sample_rate,
            _name: name,
        }
    }

    pub fn add_note(&mut self, note: Note) {
        self.notes.push(note);
    }

    pub fn play(&mut self) {
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();
        let mut current_notes = vec![];
        let mut vals = vec![];

        #[cfg(feature = "plotting")]
        let mut plotting_vals = vec![];

        #[cfg(feature = "plotting")]
        let duration = 1.1;

        #[cfg(feature = "plotting")]
        let mut plot = Plot::new(
            format!("Score {} (SR={}Hz)", self.name, self.sample_rate).as_str(),
            (0.0, duration),
            (-1.1, 1.1),
            format!(".dist/score_{}_{}.png", self.name, self.sample_rate).as_str(),
        );

        #[cfg(feature = "plotting")]
        for note in self.notes.iter() {
            let attack_values = (0..44100)
                .enumerate()
                .map(|(i, v)| {
                    let timestamp = i as f32 / 44100.0;
                    (
                        timestamp,
                        if note.generator.envelope.attack.covers(timestamp) {
                            note.generator.envelope.at(v as f32 / 44100.0)
                        } else {
                            0.0
                        },
                    )
                })
                .collect::<Vec<(f32, f32)>>();
            let decay_values = (0..44100)
                .enumerate()
                .map(|(i, v)| {
                    let timestamp = i as f32 / 44100.0;
                    (
                        timestamp,
                        if note.generator.envelope.decay.covers(timestamp) {
                            note.generator.envelope.at(v as f32 / 44100.0)
                        } else {
                            0.0
                        },
                    )
                })
                .collect::<Vec<(f32, f32)>>();

            plot.plot(attack_values, "Attack", (255, 0, 0));
            plot.plot(decay_values, "Decay", (0, 255, 0));
        }

        'builder: loop {
            self.current_sample += 1;
            let current_time = self.current_sample as f32 / self.sample_rate as f32;

            // Find currently playing notes in the binary heap and push themm to the current_notes vector
            'fetcher: loop {
                if let Some(note) = self.notes.peek() {
                    if note.start_time <= current_time {
                        current_notes.push(self.notes.pop().unwrap());
                    } else {
                        break 'fetcher;
                    }
                }
                break 'fetcher;
            }

            // Push to sink every second
            if vals.len() == self.sample_rate as usize {
                sink.append(SamplesBuffer::new(
                    1 as u16,
                    self.sample_rate as u32,
                    vals.iter().map(|(_, n)| *n).collect::<Vec<f32>>(),
                ));
                #[cfg(feature = "plotting")]
                plotting_vals.append(&mut vals);
                #[cfg(not(feature = "plotting"))]
                vals.clear();
            }

            current_notes.retain(|n| !n.is_completed(current_time));

            // If there are no notes to play, break the loop
            if current_notes.is_empty() {
                if self.notes.is_empty() {
                    sink.append(SamplesBuffer::new(
                        1 as u16,
                        self.sample_rate as u32,
                        vals.iter().map(|(_, n)| *n).collect::<Vec<f32>>(),
                    ));

                    #[cfg(feature = "plotting")]
                    plotting_vals.append(&mut vals);

                    info!(
                        "Event queue is empty ! Breaking the loop - {}",
                        current_time
                    );

                    break 'builder;
                }

                vals.push((current_time, 0.0));
                continue;
            }

            // Generate the current sample
            vals.push((
                current_time,
                current_notes
                    .iter_mut()
                    .map(|note| note.tick(self.current_sample, self.sample_rate as i32))
                    .sum(),
            ));
        }

        // #[cfg(feature = "plotting")]
        // {
        //     plot.plot(plotting_vals, "One note", (0, 0, 0));
        //     if let Err(e) = plot.render() {
        //         error!("Error while plottig: {}", e)
        //     }
        // }

        sink.sleep_until_end();
    }
}
