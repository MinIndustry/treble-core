use crate::Note;
use crate::core::envelope::prelude::{
    ADSREnvelopeBuilder, BezierSegment, ConstantSegment, LinearSegment,
};
use crate::core::generator::prelude::{
    FrequencyRelation, MixMode, MultiToneGenerator, Waveform,
    builder::{MultiToneGeneratorBuilder, ToneGeneratorBuilder},
};
use crate::instruments::Instrument;

#[derive(Default, Debug)]
pub struct Kick {
    generator: MultiToneGenerator,
    current_tick: u32,
    output: f32,
    playing: bool,
}

impl Kick {
    pub fn new() -> Self {
        Self {
            generator: MultiToneGeneratorBuilder::new()
                .add_generator(
                    ToneGeneratorBuilder::new()
                        .waveform(Waveform::WhiteNoise)
                        .frequency_relation(FrequencyRelation::Constant(1.0))
                        .amplitude_envelope(Box::new(
                            ADSREnvelopeBuilder::new()
                                .attack(Box::new(BezierSegment::new(0.0, 0.1, 0.001, (0.0, 1.0))))
                                .decay(Box::new(LinearSegment::new(0.1, 0.0, 0.1)))
                                .release(Box::new(ConstantSegment::new(0.0, Some(0.0))))
                                .build(),
                        ))
                        .build(),
                )
                .add_generator(
                    ToneGeneratorBuilder::new()
                        .waveform(Waveform::Sine)
                        .frequency_relation(FrequencyRelation::Ratio(1.0))
                        .amplitude_envelope(Box::new(ConstantSegment::new(1.0, None)))
                        .build(),
                )
                .pitch_envelope(Some(Box::from(BezierSegment::new(
                    1.0,
                    0.5,
                    0.3,
                    (2.0, 0.2),
                ))))
                .amplitude_envelope(Some(Box::new(
                    ADSREnvelopeBuilder::new()
                        .attack(Box::new(BezierSegment::new(0.0, 1.0, 0.001, (0.0, 1.0))))
                        .decay(Box::new(LinearSegment::new(1.0, 0.0, 0.3)))
                        .release(Box::new(LinearSegment::new(0.0, 0.0, 0.0)))
                        .build(),
                )))
                .mix_mode(MixMode::Sum)
                .frequency(58.0)
                .build(),
            current_tick: 0,
            output: 0.0,
            playing: false,
        }
    }
}

impl Instrument for Kick {
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
        self.output = self.generator.tick(1.0 / 44100.0);
        if self.generator.completed() {
            self.playing = false;
        }
    }
}
