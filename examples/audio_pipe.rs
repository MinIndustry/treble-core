//! Pipe and Filter Architecture test
//!! Run with `cargo run --bin pipe | uv too run pipeplot` to plot the output

use simplelog::*;
use std::fs::File;

use log::{error, info};

use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink};

use rustic::core::audio::{Block, silent_block};
use rustic::core::filters::prelude::{DelayFilter, GainFilter, Tremolo};
use rustic::core::graph::{Filter, SimpleSink, Sink as SystemSink, Source, System};

#[derive(Debug, Clone)]
struct Player {
    i: usize,
}

impl Player {
    fn new() -> Self {
        Self { i: 0 }
    }
}

impl Source for Player {
    fn pull(&mut self, block_size: usize) -> Block {
        self.i += block_size;
        silent_block(block_size)
    }
}

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

    let master_volume = 0.2;
    let sample_rate = 44100.0f32;

    let source = Box::new(Player::new());

    // "sum" node: source + delayed feedback both arrive on port 0 (mixed via Sum mode)
    let sum_node: Box<dyn Filter> = Box::from(GainFilter::new(1.0));
    // Delay of 1 second in feedback loop
    let delay_filter: Box<dyn Filter> = Box::from(DelayFilter::new(sample_rate, 1.0));
    // Diminish gain in feedback loop
    let feedback_gain: Box<dyn Filter> = Box::from(GainFilter::new(0.4));
    // Add a tremolo on the output path
    let final_tremolo: Box<dyn Filter> = Box::from(Tremolo::new(20.0, 0.5, sample_rate));

    let system_sink: Box<dyn SystemSink> = Box::from(SimpleSink::new());

    let mut system = System::new().with_block_size(512);
    let sum_node = system.add_filter(sum_node);
    let delay_filter = system.add_filter(delay_filter);
    let feedback_gain = system.add_filter(feedback_gain);
    let final_tremolo = system.add_filter(final_tremolo);

    let source_id = system.add_source(source);
    let sink_id = system.add_sink(system_sink);

    // Source feeds into the sum node (port 0)
    system.connect_source(source_id, sum_node, 0);

    // Sum node fans out: one path to delay (feedback loop), one path to output tremolo
    system.connect(sum_node, delay_filter, 0, 0);
    system.connect(sum_node, final_tremolo, 0, 0);

    // Feedback path: delay → gain → back into sum node port 0 (mixed with source)
    system.connect(delay_filter, feedback_gain, 0, 0);
    system.connect(feedback_gain, sum_node, 0, 0);

    system.connect_sink(final_tremolo, sink_id, 0);

    if system.compute().is_err() {
        error!("An error occurred while computing the filter graph's layers");
        return;
    }

    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    let mut mono_values: Vec<f32> = Vec::new();
    loop {
        mono_values.clear();
        // Run enough times to fill ~1 second of audio
        let runs = (sample_rate / system.block_size() as f32).ceil() as usize;
        for _ in 0..runs {
            system.run();
        }
        info!("Ran a second");

        match system.get_sink(0) {
            Ok(s) => {
                let frames = s.consume();
                // Mix stereo to mono for rodio
                for frame in &frames {
                    mono_values.push((frame[0] + frame[1]) * 0.5 * master_volume);
                }
            }
            Err(e) => error!("Unable to get sink from system: {}", e),
        };

        sink.append(SamplesBuffer::new(
            1_u16,
            sample_rate as u32,
            mono_values.clone(),
        ));
        while sink.len() > 5 * sample_rate as usize {}
    }
}
