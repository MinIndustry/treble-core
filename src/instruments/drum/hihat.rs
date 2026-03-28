use petgraph::prelude::NodeIndex;

use crate::Note;
use crate::core::envelope::prelude::BezierSegment;
use crate::core::filters::prelude::ResonantBandpassFilter;
use crate::core::generator::prelude::{
    MixMode, Waveform,
    builder::{MultiToneGeneratorBuilder, ToneGeneratorBuilder},
};
use crate::core::graph::SimpleSink;
use crate::core::graph::SimpleSource;
use crate::core::graph::System;
use crate::instruments::Instrument;

/// A HiHat instrument.
/// It consists of six square wave sources connected to a combinator filter. The result is then passed through a resonant bandpass filter,
/// before being shaped by an envelope generator.
#[derive(Debug)]
pub struct HiHat {
    graph: System,
    bandpass_filter_index: NodeIndex<u32>,
    playing: bool,
}

impl HiHat {
    pub fn new() -> Result<Self, String> {
        let source = SimpleSource::new(
            MultiToneGeneratorBuilder::new()
                .add_generator(
                    ToneGeneratorBuilder::new()
                        .waveform(Waveform::Square)
                        .frequency(123.0)
                        .build(),
                )
                .add_generator(
                    ToneGeneratorBuilder::new()
                        .waveform(Waveform::Square)
                        .frequency(150.0)
                        .build(),
                )
                .add_generator(
                    ToneGeneratorBuilder::new()
                        .waveform(Waveform::Square)
                        .frequency(180.0)
                        .build(),
                )
                .add_generator(
                    ToneGeneratorBuilder::new()
                        .waveform(Waveform::Square)
                        .frequency(219.0)
                        .build(),
                )
                .add_generator(
                    ToneGeneratorBuilder::new()
                        .waveform(Waveform::Square)
                        .frequency(240.0)
                        .build(),
                )
                .add_generator(
                    ToneGeneratorBuilder::new()
                        .waveform(Waveform::Square)
                        .frequency(261.0)
                        .build(),
                )
                .amplitude_envelope(Some(Box::new(BezierSegment::new(
                    1.0,
                    0.0,
                    0.5,
                    (0.0, 0.0),
                ))))
                .mix_mode(MixMode::Sum)
                .build(),
            44100.0,
        )
        .boxed();

        let mut system = System::new().with_block_size(1);
        let source_id = system.add_source(source);

        let bandpass = system.add_filter(Box::from(ResonantBandpassFilter::new(
            (10.0e3 + 400.0) / 2.0,
            20.0,
            44100.0,
        )));

        system.connect_source(source_id, bandpass, 0);
        let sink_id = system.add_sink(Box::from(SimpleSink::new()));
        system.connect_sink(bandpass, sink_id, 0);

        system
            .compute()
            .map_err(|_| "Failed to compute".to_string())?;

        Ok(Self {
            graph: system,
            bandpass_filter_index: bandpass,
            playing: false,
        })
    }
}

impl Instrument for HiHat {
    fn start_note(&mut self, _note: Note, _velocity: f32) {
        // Restart the source from the beginning of its decay envelope.
        // Immediately calling stop_source marks it as "released" so the
        // SimpleSource auto-deactivates once the envelope has fully decayed.
        self.graph.start_source(0);
        self.graph.stop_source(0);
        self.playing = true;

        // Reset bandpass filter to avoid tonal artifacts from residual state.
        if let Some(filter_box) = self.graph.get_filter_mut(self.bandpass_filter_index) {
            if let Some(bandpass) = filter_box
                .as_any_mut()
                .downcast_mut::<ResonantBandpassFilter>()
            {
                bandpass.reset();
            } else {
                log::warn!("Failed to downcast filter to ResonantBandpassFilter");
            }
        }
    }

    fn stop_note(&mut self, _note: Note) {
        // Percussive: let the decay envelope finish naturally — no hard cut.
    }

    fn into_system(self: Box<Self>) -> System {
        self.graph
    }

    fn get_output(&mut self) -> f32 {
        self.graph
            .get_sink(0)
            .unwrap()
            .consume()
            .first()
            .map(|frame| frame[0])
            .unwrap_or(0.0)
    }

    fn tick(&mut self) {
        if self.playing {
            self.graph.run();
            // Once the source has finished its decay the graph produces silence;
            // stop ticking until the next start_note to avoid wasting CPU.
            self.playing = self.graph.is_source_active(0);
        }
    }
}
