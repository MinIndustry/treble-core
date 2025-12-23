use crate::core::envelope::prelude::{ADSREnvelope, ADSREnvelopeBuilder, LinearSegment, ConstantSegment, BezierSegment};
use crate::core::envelope::Envelope;
use crate::core::generator::prelude::{
    tones::{SineWave, WhiteNoise},
    SimpleGenerator,
};
use crate::core::generator::BendableGenerator;
use crate::core::generator::ToneGenerator;
use crate::instruments::Instrument;
use crate::Note;

#[derive(Debug)]
pub struct Kick {
    generators: (Box<dyn ToneGenerator>, Box<dyn BendableGenerator>),
    envelopes: (Box<dyn Envelope>, Box<dyn Envelope>),
    pitch_curve: Box<dyn Envelope>,
    current_tick: u32,
    playing: bool,
    output: f32,
}

impl Kick {
    pub fn new() -> Self {
        Self {
            generators: (
                Box::from(WhiteNoise::new(0.02)),
                Box::from(SimpleGenerator::new(
                    Box::from(ADSREnvelope::constant()),
                    Box::from(SineWave::new(58.0, 1.0)),
                )),
            ),
            envelopes: (
                Box::from(
                    ADSREnvelopeBuilder::new()
                        .attack(Box::new(BezierSegment::new(0.0, 1.0, 0.001, (0.0, 1.0))))
                        .decay(Box::new(LinearSegment::new(1.0, 0.0, 0.1)))
                        .release(Box::new(ConstantSegment::new(0.0, Some(0.0))))
                        .build(),
                ),
                {
                    Box::from(
                        ADSREnvelopeBuilder::new()
                            .attack(Box::new(BezierSegment::new(0.0, 1.0, 0.001, (0.0, 1.0))))
                            .decay(Box::new(LinearSegment::new(1.0, 0.0, 0.5)))
                            .release(Box::new(ConstantSegment::new(0.0, Some(0.0))))
                            .build(),
                    )
                },
            ),
            pitch_curve: Box::from(BezierSegment::new(1.4, 0.1, 0.3, (2.0, 0.2))),
            current_tick: 0,
            playing: false,
            output: 0.0,
        }
    }
}

impl Instrument for Kick {
    fn start_note(&mut self, _note: Note, _velocity: f32) {
        self.current_tick = 0;
        self.playing = true;
    }

    fn stop_note(&mut self, _note: crate::Note) {
        // The note will continue playing until completed
        self.playing = false;
    }

    fn get_output(&mut self) -> f32 {
        self.output
    }

    fn tick(&mut self) {
        self.current_tick += 1;
        let current_time: f32 = self.current_tick as f32 / 44100.0;
        let current_pitch = self.pitch_curve.at(current_time, -1.0);
        self.generators.1.set_pitch_bend(current_pitch);

        self.output = self.generators.0.tick(1.0 / 44100.0)
            * self.envelopes.0.at(current_time, -1.0)
            + self.generators.1.tick(1.0 / 44100.0) * self.envelopes.1.at(current_time, -1.0);
    }
}
