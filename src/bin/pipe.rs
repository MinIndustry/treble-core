//! Pipe and Filter Architecture test
//!! Run with `cargo run --bin pipe | uv too run pipeplot` to plot the output

use log::{error, info, trace};

use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink};

use rustic::core::envelope::prelude::ADSREnvelope;
use rustic::core::filters::prelude::{
    CombinatorFilter, DelayFilter, DuplicateFilter, GainFilter, Tremolo,
};
use rustic::core::generator::GENERATORS;
use rustic::core::graph::{
    AudioGraphElement, Filter, SimpleSink, Sink as SystemSink, Source, System,
};
use rustic::core::note::Note;
use rustic::core::tones::{NOTES, TONES_FREQ};

#[derive(Debug)]
struct Player {
    notes: Vec<Note>,
    i: usize,
    sample_rate: f32,
}

impl Player {
    fn new() -> Self {
        let envelope = ADSREnvelope::new()
            .with_attack(0.1, 1.0, None)
            .with_decay(0.0, 1.0, None)
            .with_release(20.0, 0.0, None);

        let initial_note = Note::new(TONES_FREQ[NOTES::C as usize][4], 0.0, 0.2)
            .with_generator(GENERATORS::SINE)
            .with_envelope(Box::from(envelope.clone()));
        let second_note = Note::new(TONES_FREQ[NOTES::D as usize][3], 2.0, 0.2)
            .with_generator(GENERATORS::SINE)
            .with_envelope(Box::from(envelope.clone()));
        let third_note = Note::new(TONES_FREQ[NOTES::FS as usize][5], 4.0, 0.2)
            .with_generator(GENERATORS::SINE)
            .with_envelope(Box::from(envelope.clone()));

        let notes = vec![initial_note, second_note, third_note];

        Self {
            notes,
            i: 0,
            sample_rate: 44100.0,
        }
    }
}

impl Source for Player {
    fn pull(&mut self) -> f32 {
        trace!("Player::pull");
        self.i += 1;
        self.notes[0].tick(self.sample_rate as i32)
            + self.notes[1].tick(self.sample_rate as i32)
            + self.notes[2].tick(self.sample_rate as i32)
    }
}

impl AudioGraphElement for Player {
    fn get_name(&self) -> &str {
        "Player"
    }

    fn set_index(&mut self, _index: usize) {}

    fn get_index(&self) -> usize {
        0
    }
}

fn main() {
    colog::init();

    let master_volume = 0.2;
    let sample_rate = 44100.0; // 44100 Hz

    let source = Box::new(Player::new());

    let dupe_filter: Box<dyn Filter> = Box::from(DuplicateFilter::new());
    let sum_filter: Box<dyn Filter> = Box::from(CombinatorFilter::new(2, 1));

    // Delay of half a second
    let delay_filter: Box<dyn Filter> = Box::from(DelayFilter::new(sample_rate, 0.5));

    // Diminish gain in feedback loop
    let gain_filter: Box<dyn Filter> = Box::from(GainFilter::new(0.99));

    // Add a tremolo
    let final_tremolo: Box<dyn Filter> = Box::from(Tremolo::new(5.0, 0.4, 0.6));
    // let clipper: Box<dyn Filter> = Box::from(Clipper::new(0.75));

    let system_sink: Box<dyn SystemSink> = Box::from(SimpleSink::new());

    let mut system = System::<1, 1>::new();
    let sum_filter = system.add_filter(sum_filter);
    let dupe_filter = system.add_filter(dupe_filter);
    let delay_filter = system.add_filter(delay_filter);
    let gain_filter = system.add_filter(gain_filter);
    let final_tremolo = system.add_filter(final_tremolo);
    // let clipper = system.add_filter(clipper);

    system.set_source(0, source);
    system.set_sink(0, system_sink);

    system.connect(sum_filter, dupe_filter, 0, 0);
    system.connect(dupe_filter, delay_filter, 1, 0);
    system.connect(delay_filter, gain_filter, 0, 0);
    system.connect(gain_filter, sum_filter, 0, 1);

    system.connect(dupe_filter, final_tremolo, 0, 0);

    system.connect_sink(final_tremolo, 0, 0);
    system.connect_source(0, sum_filter, 0);

    if let Err(_) = system.compute() {
        error!("An error occured while computing the filter graph's layers");
        return;
    }

    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    // TODO: Move system to score
    let mut values = vec![];
    loop {
        values.clear();
        for _ in 0..sample_rate as usize {
            system.run();
        }
        info!("Ran a second");

        match system.get_sink(0) {
            Ok(s) => values.append(&mut s.consume(sample_rate as usize)),
            Err(e) => error!("Unable to get sink from system: {}", e),
        };

        sink.append(SamplesBuffer::new(
            1 as u16,
            sample_rate as u32,
            values
                .iter()
                .map(|n| *n * master_volume)
                .collect::<Vec<f32>>(),
        ));
        while sink.len() > 5 * sample_rate as usize {}
    }
}
