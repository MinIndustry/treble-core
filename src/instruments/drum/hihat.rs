use crate::core::envelope::prelude::{BezierSegment, ConstantSegment};
use crate::core::envelope::Envelope;
use crate::core::filters::prelude::{CombinatorFilter, GainFilter, ResonantBandpassFilter};
use crate::core::graph::simple_source;
use crate::core::graph::SimpleSink;
use crate::core::graph::System;
use crate::instruments::Instrument;
use crate::Note;

use crate::core::generator::prelude::{builder::{ToneGeneratorBuilder, CompositeGeneratorBuilder}, Waveform};

use petgraph::prelude::NodeIndex;

#[cfg(debug_assertions)]
use std::fs::File;
#[cfg(debug_assertions)]
use std::io::Write;

/// A HiHat instrument.
/// It consists of six square wave sources connected to a combinator filter. The result is then passed through a resonant bandpass filter,
/// before being shaped by an envelope generator.
#[derive(Debug)]
pub struct HiHat {
    graph: System<1, 1>,
    bandpass_filter_index: NodeIndex<u32>,
    playing: bool,
    time: f32,
    amplitude_envelope: Box<dyn Envelope>,
    #[cfg(debug_assertions)]
    output_buffer: File,
}

impl HiHat {
    pub fn new() -> Result<Self, String> {
        let source = simple_source(CompositeGeneratorBuilder::new()
            .add_generator(Box::new(ToneGeneratorBuilder::new()
                .waveform(Waveform::Square)
                .frequency(123.0)
                .build()))
            .add_generator(Box::new(ToneGeneratorBuilder::new()
                .waveform(Waveform::Square)
                .frequency(150.0)
                .build()))
            .add_generator(Box::new(ToneGeneratorBuilder::new()
                .waveform(Waveform::Square)
                .frequency(180.0)
                .build()))
            .add_generator(Box::new(ToneGeneratorBuilder::new()
                .waveform(Waveform::Square)
                .frequency(219.0)
                .build()))
            .add_generator(Box::new(ToneGeneratorBuilder::new()
                .waveform(Waveform::Square)
                .frequency(240.0)
                .build()))
            .add_generator(Box::new(ToneGeneratorBuilder::new()
                .waveform(Waveform::Square)
                .frequency(261.0)
                .build()))
            .amplitude_envelope(Some(Box::new(ConstantSegment::new(1.0, None))))
            .build());

        let mut system = System::<1, 1>::new();
        system.set_source(0, source);

        let bandpass = system.add_filter(Box::from(ResonantBandpassFilter::new(
            (10.0e3 + 400.0) / 2.0,
            20.0,
            44100.0,
        )));

        system.connect_source(0, bandpass, 0);

        let sink: SimpleSink = SimpleSink::new();
        system.set_sink(0, Box::from(sink));
        system.connect_sink(bandpass, 0, 0);

        system
            .compute()
            .map_err(|_| "Failed to compute".to_string())?;

        match crate::app::prelude::FSConfig::debug_dir("HiHat", "hihat.viz") {
            Ok(path) => {
                if let Err(e) = system.save_to_file(&path) {
                    log::warn!("Failed to save visualization: {}", e);
                }
            }
            Err(_) => log::warn!("Failed to build path to save hihat graph"),
        }

        let amplitude_envelope = Box::new(BezierSegment::new(4.0, 0.0, 0.2, (0.0, 0.0)));

        #[cfg(debug_assertions)]
        let output_path = crate::app::prelude::FSConfig::debug_dir("HiHat", "hihat_output.txt").unwrap();

        Ok(Self {
            graph: system,
            bandpass_filter_index: bandpass,
            playing: false,
            amplitude_envelope,
            time: 0.0,
            #[cfg(debug_assertions)]
            output_buffer: File::create(output_path).unwrap(),
        })
    }
}

impl Instrument for HiHat {
    fn start_note(&mut self, _note: Note, _velocity: f32) {
        log::trace!("Starting HiHat note");
        self.playing = true;
        self.time = 0.0;

        // Reset the bandpass filter state to ensure clean retriggering.
        // For percussive sounds, residual filter state from previous hits
        // causes tonal artifacts and reduces transient clarity.
        if let Some(filter_box) = self.graph.get_filter_mut(self.bandpass_filter_index) {
            // Downcast to ResonantBandpassFilter to access its reset method
            if let Some(bandpass) = filter_box.as_any_mut().downcast_mut::<ResonantBandpassFilter>() {
                bandpass.reset();
            } else {
                log::warn!("Failed to downcast filter to ResonantBandpassFilter");
            }
        }
    }

    fn stop_note(&mut self, _note: Note) {
        log::trace!("Stopping HiHat note");
        self.playing = false;
    }

    fn get_output(&mut self) -> f32 {
        let value = *self
            .graph
            .get_sink(0)
            .unwrap()
            .consume(1)
            .first()
            .unwrap_or(&0.0);
        #[cfg(debug_assertions)]
        {
            // Check if the output buffer is empty
            if self.output_buffer.metadata().unwrap().len() > 0 {
                if let Err(e) = self.output_buffer.write(format!(" {}", value).as_bytes()) {
                    log::warn!("Failed to write to output buffer: {}", e);
                }
            } else {
                if let Err(e) = self.output_buffer.write(format!("{}", value).as_bytes()) {
                    log::warn!("Failed to write to output buffer: {}", e);
                }
            }
        }
        value * self.amplitude_envelope.at(self.time, -1.0)
    }

    fn tick(&mut self) {
        if self.playing {
            self.graph.run();
            self.time += 1.0 / 44100.0;
        }
    }
}
