use crate::core::envelope::prelude::{ADSREnvelopeBuilder, LinearSegment, ConstantSegment, BezierSegment};
use crate::core::generator::prelude::{builder::{ToneGeneratorBuilder, CompositeGeneratorBuilder}, Waveform, MixMode, FrequencyRelation, MultiToneGenerator};
use crate::instruments::Instrument;
use crate::Note;

#[derive(Debug)]
pub struct Kick {
    generator: Box<dyn MultiToneGenerator>,
    current_tick: u32,
    output: f32,
}

impl Kick {
    pub fn new() -> Self {
        Self {
            generator: Box::new(CompositeGeneratorBuilder::new()
                .add_generator(
                    Box::new(ToneGeneratorBuilder::new()
                        .waveform(Waveform::WhiteNoise)
                        .frequency_relation(FrequencyRelation::Constant(1.0))
                        .amplitude_envelope(
                            Box::new(ADSREnvelopeBuilder::new()
                            .attack(Box::new(BezierSegment::new(0.0, 0.1, 0.001, (0.0, 1.0))))
                            .decay(Box::new(LinearSegment::new(0.1, 0.0, 0.1)))
                            .release(Box::new(ConstantSegment::new(0.0, Some(0.0))))
                            .build())
                        )
                        .build()))
                .add_generator(
                    Box::new(ToneGeneratorBuilder::new()
                        .waveform(Waveform::Sine)
                        .frequency_relation(FrequencyRelation::Ratio(1.0))
                        .amplitude_envelope(
                            Box::new(ConstantSegment::new(1.0, None)))
                        .build()))
                .pitch_envelope(Some(Box::from(BezierSegment::new(1.0, 0.5, 0.3, (2.0, 0.2)))))
                .mix_mode(MixMode::Sum)
                .frequency(58.0)
                .build()),
            current_tick: 0,
            output: 0.0,
        }
    }
}

impl Instrument for Kick {
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
        self.output = self.generator.tick(1.0 / 44100.0);
    }
}
