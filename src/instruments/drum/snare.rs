use crate::Note;
use crate::core::envelope::prelude::{
    ADSREnvelopeBuilder, BezierSegment, ConstantSegment, LinearSegment,
};
use crate::core::filters::prelude::GainFilter;
use crate::core::generator::prelude::{
    FrequencyRelation, MixMode, MultiToneGenerator, Waveform,
    builder::{MultiToneGeneratorBuilder, ToneGeneratorBuilder},
};
use crate::core::graph::{MonophonicAllocationStrategy, MonophonicSource, SimpleSink, System};
use crate::instruments::Instrument;

/// A snare for the drum kit
#[derive(Debug)]
pub struct Snare {
    generator: MultiToneGenerator,
    current_tick: u32,
    output: f32,
    playing: bool,
}

impl Default for Snare {
    fn default() -> Self {
        Self::new()
    }
}

impl Snare {
    pub fn new() -> Self {
        Self {
            generator: MultiToneGeneratorBuilder::new()
                .add_generator(
                    ToneGeneratorBuilder::new()
                        .waveform(Waveform::WhiteNoise)
                        .frequency_relation(FrequencyRelation::Constant(1.0)) // Frequency is irrelevant for noise. This is to avoid warnings
                        .amplitude_envelope(Box::new(ConstantSegment::new(0.1, None)))
                        .build(),
                )
                .add_generator(
                    ToneGeneratorBuilder::new()
                        .waveform(Waveform::Sine)
                        .frequency_relation(FrequencyRelation::Ratio(1.0))
                        .build(),
                )
                .amplitude_envelope(
                    // TODO: Test envelope segments
                    Some(Box::new(
                        ADSREnvelopeBuilder::new()
                            .attack(Box::new(BezierSegment::new(0.0, 1.0, 0.001, (0.0, 1.0))))
                            .decay(Box::new(LinearSegment::new(1.0, 0.0, 0.2)))
                            .release(Box::new(LinearSegment::new(0.0, 0.0, 0.0)))
                            .build(),
                    )),
                )
                .pitch_envelope(Some(Box::from(BezierSegment::new(
                    1.2,
                    0.8,
                    0.2,
                    (0.0, 1.0),
                ))))
                .mix_mode(MixMode::Sum)
                .frequency(158.0)
                .build(),
            current_tick: 0,
            output: 0.0,
            playing: false,
        }
    }
}

impl Instrument for Snare {
    fn start_note(&mut self, _note: Note, _velocity: f32) {
        self.current_tick = 0;
        self.playing = true;
        self.generator.start();
    }

    fn stop_note(&mut self, _note: Note) {
        self.generator.stop();
    }

    fn get_output(&mut self) -> f32 {
        self.output
    }

    fn tick(&mut self) {
        if !self.playing {
            self.output = 0.0;
            return;
        }
        self.current_tick += 1;
        self.output = self.generator.tick(1.0 / 44100.0);
        if self.generator.completed() {
            self.playing = false;
        }
    }

    fn into_system(self: Box<Self>) -> System {
        let source = MonophonicSource::new_percussive(self.generator, 44100.0, MonophonicAllocationStrategy::Replace);
        let mut system = System::new();
        let source_idx = system.add_source(Box::new(source));
        let output = system.add_filter(Box::new(GainFilter::new(1.0)));
        system.connect_source(source_idx, output, 0);
        let sink_idx = system.add_sink(Box::new(SimpleSink::new()));
        system.connect_sink(output, sink_idx, 0);
        system.compute().expect("Snare system compute failed");
        system
    }
}
