use super::{
    Bendable, BendableGenerator, Envelope, EnvelopedGenerator, FrequencyTransition, Generator,
    KeyboardGenerator, Segment, VariableToneGenerator,
};

#[derive(Debug)]
/// A generic generator that contains a tone generator,
/// amplitude & pitch envelopes
pub struct SimpleGenerator {
    pub envelope: Box<dyn Envelope + Send + Sync>, // An envelope for the note amplitude
    pitch_curve: Segment,                          // An evelope for the note pitch
    pitch_bend: f32,                               // When negative no pitch bend
    tone_generator: Box<dyn VariableToneGenerator + Send + Sync>,
    timer: f32,
    stop_timestamp: f32,
}

impl SimpleGenerator {
    /// Creates a new generator with the given envelopes and tone generator.
    pub fn new(
        envelope: Box<dyn Envelope + Send + Sync>,
        tone_generator: Box<dyn VariableToneGenerator + Send + Sync>,
    ) -> SimpleGenerator {
        Self {
            envelope,
            pitch_curve: Segment::default(),
            tone_generator,
            pitch_bend: 0.0,
            timer: 0.0,
            stop_timestamp: -1.0,
        }
    }

    pub fn is_stopping(&self) -> bool {
        return self.stop_timestamp >= 0.0;
    }

    pub fn set_tone_generator(
        &mut self,
        tone_generator: Box<dyn VariableToneGenerator + Send + Sync>,
    ) {
        self.tone_generator = tone_generator;
    }

    pub fn set_pitch_bend(&mut self, pitch_curve: Segment) {
        self.pitch_curve = pitch_curve
    }

    pub fn set_ampl_envelope(&mut self, ampl_envelope: Box<dyn Envelope + Send + Sync>) {
        self.envelope = ampl_envelope
    }
}

impl Generator for SimpleGenerator {
    fn start(&mut self) {
        self.timer = 0.0;
        self.stop_timestamp = -1.0;
    }

    fn stop(&mut self) {
        self.stop_timestamp = self.timer;
    }

    /// Returns the note value at a point in time, given the note_on, note_off and current time.
    fn tick(&mut self, elapsed_time: f32) -> f32 {
        let warp = if self.pitch_curve.covers(self.timer) {
            self.pitch_curve.at(self.timer)
        } else {
            1.0
        };
        self.timer += warp * elapsed_time;

        let ampl = self.envelope.at(self.timer, self.stop_timestamp);
        let sample: f32 = self.tone_generator.tick(self.timer);

        ampl * sample
    }

    fn completed(&self) -> bool {
        if self.stop_timestamp > 0.0 {
            self.envelope.completed(self.timer, self.stop_timestamp)
        } else {
            false
        }
    }

    fn set_frequency(&mut self, frequency: f32) {
        self.tone_generator
            .change_frequency(frequency, FrequencyTransition::DIRECT);
    }
}

impl EnvelopedGenerator for SimpleGenerator {
    fn set_envelope(&mut self, envelope: Box<dyn Envelope + Send + Sync>) {
        self.envelope = envelope;
    }
}

impl Bendable for SimpleGenerator {
    fn set_pitch_bend(&mut self, pitch: f32) {
        self.pitch_bend = pitch;
    }
}

impl BendableGenerator for SimpleGenerator {}

impl KeyboardGenerator for SimpleGenerator {}
