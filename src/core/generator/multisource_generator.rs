use super::{
    Bendable, BendableGenerator, Envelope, EnvelopedGenerator, FrequencyTransition, Generator,
    KeyboardGenerator, Segment, VariableToneGenerator,
};

#[derive(Debug)]
/// A generator with more complex patterns, allowing for
/// multiple generators at once. The amplitude and pitch envelopes
/// are shared between all generators.
pub struct MultiSourceGenerator {
    pub envelope: Box<dyn Envelope + Send + Sync>, // An envelope for the note amplitude
    pitch_curve: Segment,                          // An evelope for the note pitch
    pitch_bend: f32,                               // When negative no pitch bend
    tone_generators: Vec<(Box<dyn VariableToneGenerator + Send + Sync>, f32)>,
    timer: f32,
    stop_timestamp: f32,
}

impl MultiSourceGenerator {
    /// Creates a new generator with the given envelopes and tone generator.
    pub fn new(
        envelope: Box<dyn Envelope + Send + Sync>,
        tone_generator: Box<dyn VariableToneGenerator + Send + Sync>,
    ) -> MultiSourceGenerator {
        Self {
            envelope,
            pitch_curve: Segment::default(),
            tone_generators: vec![(tone_generator, 1.0)],
            pitch_bend: 0.0,
            timer: 0.0,
            stop_timestamp: -1.0,
        }
    }

    pub fn add_generator(
        mut self,
        tone_generator: Box<dyn VariableToneGenerator + Send + Sync>,
        factor: f32,
    ) -> Self {
        self.tone_generators.push((tone_generator, factor));
        self
    }

    pub fn set_pitch_bend(&mut self, pitch_curve: Segment) {
        self.pitch_curve = pitch_curve
    }

    pub fn set_ampl_envelope(&mut self, ampl_envelope: Box<dyn Envelope + Send + Sync>) {
        self.envelope = ampl_envelope
    }
}

impl Generator for MultiSourceGenerator {
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
        let sample: f32 = self
            .tone_generators
            .iter_mut()
            .map(|(g, w)| g.tick(self.timer) * *w)
            .sum::<f32>();

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
        self.tone_generators
            .iter_mut()
            .for_each(|g| g.0.change_frequency(frequency, FrequencyTransition::DIRECT));
    }
}

impl EnvelopedGenerator for MultiSourceGenerator {
    fn set_envelope(&mut self, envelope: Box<dyn Envelope + Send + Sync>) {
        self.envelope = envelope;
    }
}

impl Bendable for MultiSourceGenerator {
    fn set_pitch_bend(&mut self, pitch: f32) {
        self.pitch_bend = pitch;
    }
}

impl BendableGenerator for MultiSourceGenerator {}

impl KeyboardGenerator for MultiSourceGenerator {}
