use std::cmp::Ordering;

use super::envelope::prelude::*;
use super::envelope::Envelope;
use super::generator::prelude::*;
use super::generator::{Generator, VariableToneGenerator, GENERATORS};

/// Represents a musical note that can be part of a score. It has an associated generator,
/// that can generate the tone of the note in any of the `GENERATORS` shapes.
///
/// # Fields
///
/// * `frequency` - The frequency of the note in Hertz (Hz).
/// * `start_time` - The start time of the note in seconds.
/// * `duration` - The duration of the note in seconds.
/// * `generator` - A boxed trait object that generates the tone for the note.
#[derive(Debug)]
pub struct Note {
    pub frequency: f32,
    pub start_time: f32,
    pub duration: f32,
    pub generator: Box<SimpleGenerator>,
}

impl Eq for Note {}

impl PartialEq for Note {
    fn eq(&self, other: &Self) -> bool {
        self.start_time == other.start_time && self.frequency == other.frequency
    }
}

impl PartialOrd for Note {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.start_time.partial_cmp(&self.start_time)
    }
}

impl Ord for Note {
    fn cmp(&self, other: &Self) -> Ordering {
        other.partial_cmp(&self).unwrap_or(Ordering::Equal)
    }
}

impl Note {
    /// Creates a new note instance with the given parameters. The default generator is a Sine generator.
    pub fn new(frequency: f32, start_time: f32, duration: f32) -> Self {
        let env: ADSREnvelope = ADSREnvelope::constant();
        let tone_generator: Box<dyn VariableToneGenerator> =
            Box::from(SineWave::new(frequency, 1.0));

        Self {
            frequency,
            start_time,
            duration,
            generator: Box::from(env.attach_amplitude(tone_generator)),
        }
    }

    /// Sets the tone generator for the note and returns the modified note.
    ///
    /// This method allows you to specify the type of tone generator to be used for the note.
    /// It takes a `GENERATORS` enum value and sets the corresponding generator.
    ///
    /// # Arguments
    ///
    /// * `generator` - The type of tone generator to be used for the note.
    ///
    /// # Returns
    ///
    /// The modified `Note` instance with the specified tone generator.
    ///
    /// # Examples
    ///
    /// ```
    /// use rustic::core::note::Note;
    /// use rustic::core::generator::GENERATORS;
    ///
    /// let note = Note::new(440.0, 0.0, 1.0)
    ///     .with_generator(GENERATORS::SINE);
    /// ```
    pub fn with_generator(mut self, generator: GENERATORS) -> Self {
        let new_tone_generator: Box<dyn VariableToneGenerator> = match generator {
            GENERATORS::SAW => Box::from(SawTooth::new(self.frequency, 1.0)),
            GENERATORS::SINE => Box::from(SineWave::new(self.frequency, 1.0)),
            GENERATORS::SQUARE => Box::from(SquareWave::new(self.frequency, 1.0)),
            GENERATORS::NOISE => Box::from(WhiteNoise::new(1.0)),
            GENERATORS::NULL => Box::from(NullGenerator::new()),
        };

        self.generator.set_tone_generator(new_tone_generator);
        self
    }

    /// Sets the envelope for the note and returns the modified note
    ///
    /// This method allows you to attach an envelope to the note. This envelope will
    /// affect the amplitude of the note (the volume).
    ///
    /// # Arguments
    ///
    /// * `envelope` - A reference to the envelope to attach to the note. The instance
    /// that this reference points to is cloned then attached to the note.
    ///
    /// # Returns
    ///
    /// The modified `Note` instance with the specified envelope.
    ///
    /// # Example
    ///
    /// ```
    /// use rustic::core::note::Note;
    /// use rustic::core::envelope::prelude::ADSREnvelope;
    ///
    /// let note = Note::new(440.0, 0.0, 1.0)
    ///     .with_envelope(Box::new(ADSREnvelope::constant()));
    /// ```
    pub fn with_envelope(mut self, envelope: Box<dyn Envelope + Send + Sync>) -> Self {
        self.generator.set_ampl_envelope(envelope);
        self
    }

    /// Sets the pitch bend for the note and returns the modified note
    /// This method allows you to attach a pitch bend to the note. This pitch bend will
    /// affect the frequency of the note.
    /// # Arguments
    /// * `pitch_bend` - A reference to the pitch bend to attach to the note. The instance
    /// that this reference points to is cloned then attached to the note.
    /// # Returns
    /// The modified `Note` instance with the specified pitch bend.
    /// # Example
    /// ```
    /// use rustic::core::note::Note;
    /// use rustic::core::envelope::prelude::Segment;
    ///
    /// let note = Note::new(440.0, 0.0, 1.0)
    ///    .with_pitch_bend(&Segment::new(0.0, 1.0, 440.0, 880.0, None));
    /// ```
    pub fn with_pitch_bend(mut self, pitch_bend: &Segment) -> Self {
        self.generator.set_pitch_bend(pitch_bend.clone());
        self
    }

    pub fn tick(&mut self, sample_rate: i32) -> f32 {
        self.generator.tick(1.0 / sample_rate as f32)
    }

    /// Returns true when to note is completed (amplitude envelope release has finished)
    /// This depends on the `off_time` of the note.
    pub fn is_completed(&mut self, time: f32) -> bool {
        if time > self.start_time + self.duration && !self.generator.is_stopping() {
            self.generator.stop();
        }

        self.generator
            .envelope
            .completed(time, self.start_time + self.duration)
            && time > self.start_time
    }
}
