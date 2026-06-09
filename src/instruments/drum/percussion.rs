use crate::Note;
use crate::core::envelope::prelude::{
    ADSREnvelopeBuilder, BezierSegment, ConstantSegment, LinearSegment,
};
use crate::core::filters::prelude::GainFilter;
use crate::core::generator::prelude::{
    FrequencyRelation, MultiToneGenerator, Waveform,
    builder::{MultiToneGeneratorBuilder, ToneGeneratorBuilder},
};
use crate::core::graph::{MonophonicAllocationStrategy, MonophonicSource, SimpleSink, System};
use crate::instruments::Instrument;

/// Handclap — short band-limited noise burst.
#[derive(Debug, Default)]
pub struct Clap {
    generator: MultiToneGenerator,
    output: f32,
    playing: bool,
}

impl Clap {
    pub fn new() -> Self {
        Self {
            generator: MultiToneGeneratorBuilder::new()
                .add_generator(
                    ToneGeneratorBuilder::new()
                        .waveform(Waveform::WhiteNoise)
                        .frequency_relation(FrequencyRelation::Constant(1.0))
                        .amplitude_envelope(Box::new(
                            ADSREnvelopeBuilder::new()
                                .attack(Box::new(BezierSegment::new(0.0, 1.0, 0.001, (0.0, 1.0))))
                                .decay(Box::new(LinearSegment::new(1.0, 0.0, 0.06)))
                                .release(Box::new(ConstantSegment::new(0.0, Some(0.0))))
                                .build(),
                        ))
                        .build(),
                )
                .build(),
            output: 0.0,
            playing: false,
        }
    }
}

impl Instrument for Clap {
    fn start_note(&mut self, _note: Note, _velocity: f32) {
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
        self.output = self.generator.tick(1.0 / 44100.0);
        if self.generator.completed() {
            self.playing = false;
        }
    }

    fn into_system(self: Box<Self>, sample_rate: f32) -> System {
        percussive_system(self.generator, sample_rate, 0.9)
    }
}

/// Rimshot — short pitched sine ping.
#[derive(Debug, Default)]
pub struct Rim {
    generator: MultiToneGenerator,
    output: f32,
    playing: bool,
}

impl Rim {
    pub fn new() -> Self {
        Self {
            generator: MultiToneGeneratorBuilder::new()
                .add_generator(
                    ToneGeneratorBuilder::new()
                        .waveform(Waveform::Sine)
                        .frequency_relation(FrequencyRelation::Ratio(1.0))
                        .build(),
                )
                .amplitude_envelope(Some(Box::new(
                    ADSREnvelopeBuilder::new()
                        .attack(Box::new(BezierSegment::new(0.0, 1.0, 0.001, (0.0, 1.0))))
                        .decay(Box::new(LinearSegment::new(1.0, 0.0, 0.04)))
                        .release(Box::new(ConstantSegment::new(0.0, Some(0.0))))
                        .build(),
                )))
                .frequency(800.0)
                .build(),
            output: 0.0,
            playing: false,
        }
    }
}

impl Instrument for Rim {
    fn start_note(&mut self, _note: Note, _velocity: f32) {
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
        self.output = self.generator.tick(1.0 / 44100.0);
        if self.generator.completed() {
            self.playing = false;
        }
    }

    fn into_system(self: Box<Self>, sample_rate: f32) -> System {
        percussive_system(self.generator, sample_rate, 0.85)
    }
}

/// Tom — pitched drum with pitch drop (similar to kick, higher base frequency).
#[derive(Debug, Default)]
pub struct Tom {
    generator: MultiToneGenerator,
    output: f32,
    playing: bool,
}

impl Tom {
    pub fn new() -> Self {
        Self {
            generator: MultiToneGeneratorBuilder::new()
                .add_generator(
                    ToneGeneratorBuilder::new()
                        .waveform(Waveform::Sine)
                        .frequency_relation(FrequencyRelation::Ratio(1.0))
                        .build(),
                )
                .amplitude_envelope(Some(Box::new(
                    ADSREnvelopeBuilder::new()
                        .attack(Box::new(BezierSegment::new(0.0, 1.0, 0.002, (0.0, 1.0))))
                        .decay(Box::new(LinearSegment::new(1.0, 0.0, 0.25)))
                        .release(Box::new(ConstantSegment::new(0.0, Some(0.0))))
                        .build(),
                )))
                .pitch_envelope(Some(Box::from(BezierSegment::new(
                    1.5,
                    0.6,
                    0.15,
                    (0.0, 1.0),
                ))))
                .frequency(120.0)
                .build(),
            output: 0.0,
            playing: false,
        }
    }
}

impl Instrument for Tom {
    fn start_note(&mut self, _note: Note, _velocity: f32) {
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
        self.output = self.generator.tick(1.0 / 44100.0);
        if self.generator.completed() {
            self.playing = false;
        }
    }

    fn into_system(self: Box<Self>, sample_rate: f32) -> System {
        percussive_system(self.generator, sample_rate, 0.95)
    }
}

fn percussive_system(generator: MultiToneGenerator, sample_rate: f32, gain: f32) -> System {
    let source = MonophonicSource::new_percussive(
        generator,
        sample_rate,
        MonophonicAllocationStrategy::Replace,
    );
    let mut system = System::new();
    let source_idx = system.add_source(Box::new(source));
    let gain_idx = system.add_filter(Box::new(GainFilter::new(gain)));
    system.connect_source(source_idx, gain_idx, 0);
    let sink_idx = system.add_sink(Box::new(SimpleSink::new()));
    system.connect_sink(gain_idx, sink_idx, 0);
    system.compute().expect("percussive system compute failed");
    system
}
