use crate::core::envelope::prelude::{ADSREnvelopeBuilder, BezierSegment, LinearSegment};
use crate::core::generator::prelude::{
    builder::{CompositeGeneratorBuilder, ToneGeneratorBuilder},
    FrequencyRelation,
    MixMode,
    MultiToneGenerator,
    Waveform
};
use crate::instruments::Instrument;
use crate::Note;

/// A snare for the drum kit
#[derive(Debug)]
pub struct Snare {
    generator: Box<dyn MultiToneGenerator>,
    current_tick: u32,
    output: f32,
}

impl Snare {
    pub fn new() -> Self {
        Self {
            generator: Box::new(CompositeGeneratorBuilder::new()
                .add_generator(
                    Box::new(ToneGeneratorBuilder::new()
                        .waveform(Waveform::WhiteNoise)
                        .frequency_relation(FrequencyRelation::Constant(1.0))  // Frequency is irrelevant for noise. This is to avoid warnings
                        .amplitude_envelope(
                            Box::new(ADSREnvelopeBuilder::new()
                                .attack(Box::new(BezierSegment::new(0.0, 1.0, 0.001, (0.0, 1.0))))
                                .decay(Box::new(LinearSegment::new(1.0, 0.0, 0.3)))
                                .release(Box::new(LinearSegment::new(0.0, 0.0, 0.0)))
                                .build()))
                        .build())
                )
                .add_generator(
                    Box::new(ToneGeneratorBuilder::new()
                        .waveform(Waveform::Sine)
                        .frequency_relation(FrequencyRelation::Ratio(1.0))
                        .amplitude_envelope(
                            Box::new(ADSREnvelopeBuilder::new()
                                .attack(Box::new(BezierSegment::new(0.0, 1.0, 0.001, (0.0, 1.0))))
                                .decay(Box::new(LinearSegment::new(1.0, 0.0, 0.5)))
                                .release(Box::new(LinearSegment::new(0.0, 0.0, 0.0)))
                                .build()))
                        .build())
                )
                .pitch_envelope(Some(
                    Box::from(BezierSegment::new(1.2, 1.0, 0.3, (0.0, 1.0)))
                ))
                .mix_mode(MixMode::Sum)
                .frequency(58.0)
                .build()),
            current_tick: 0,
            output: 0.0,
        }
    }
}

impl Instrument for Snare {
    fn start_note(&mut self, _note: Note, _velocity: f32) {
        self.current_tick = 0;
        self.generator.start();
    }

    fn stop_note(&mut self, _note: crate::Note) {
        // The note will continue playing until completed
        self.generator.stop();
    }

    fn get_output(&mut self) -> f32 {
        self.output
    }

    fn tick(&mut self) {
        self.current_tick += 1;

        self.output = self.generator.tick(1.0 / 44100.0);
    }
}
