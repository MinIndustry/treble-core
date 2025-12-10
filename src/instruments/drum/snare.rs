use crate::core::envelope::prelude::{ADSREnvelope, Segment};
use crate::core::envelope::Envelope;
use crate::core::generator::prelude::{
    tones::{SineWave, WhiteNoise},
    SimpleGenerator, ToneGenerator,
};
use crate::core::generator::BendableGenerator;
use crate::instruments::Instrument;
use crate::Note;

/// A snare for the drum kit
#[derive(Debug)]
pub struct Snare {
    generators: (Box<dyn ToneGenerator>, Box<dyn BendableGenerator>),
    envelopes: (Box<dyn Envelope>, Box<dyn Envelope>),
    pitch_curve: Box<dyn Envelope>,
    current_tick: u32,
    playing: bool,
    output: f32,
}

impl Snare {
    pub fn new() -> Self {
        Self {
            generators: (
                Box::from(WhiteNoise::new(0.5)),
                Box::from(SimpleGenerator::new(
                    Box::from(ADSREnvelope::constant()),
                    Box::from(SineWave::new(58.0, 1.0)),
                )),
            ),
            envelopes: (
                Box::from(
                    ADSREnvelope::new()
                        .with_attack(0.001, 1.0, Some((0.0, 1.0)))
                        .with_decay(0.3, 0.0, None)
                        .with_release(0.0, 0.0, Some((0.0, 0.0))),
                ),
                {
                    Box::from(
                        ADSREnvelope::new()
                            .with_attack(0.001, 1.0, Some((0.0, 1.0)))
                            .with_decay(0.5, 0.0, None)
                            .with_release(0.0, 0.0, Some((0.0, 0.0))),
                    )
                },
            ),
            pitch_curve: Box::from(Segment::new(1.2, 1.0, 0.3, 0.0, Some((0.0, 1.0)))),
            current_tick: 0,
            playing: false,
            output: 0.0,
        }
    }
}

impl Instrument for Snare {
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
