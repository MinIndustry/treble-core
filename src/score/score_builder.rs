use crate::{prelude::Instrument, score::{score::{Score, TimeSignature}, staff::Staff}};

pub struct ScoreBuilder {
    tempo: usize,
    signature: TimeSignature,
    name: String,
    instruments: Vec<Box<dyn Instrument>>,
    staves: Vec<Staff>
}

impl Default for ScoreBuilder {
    fn default() -> Self {
        Self {
            tempo: 120,
            signature: TimeSignature::C,
            name: String::from("New score"),
            instruments: Vec::new(),
            staves: Vec::new()
        }
    }
}

impl ScoreBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds the ScoreBuilder into a Score. This consumes
    /// the builder
    pub fn build(self) -> Score {
        Score::new(
            self.name,
            self.signature,
            self.tempo,
            self.instruments,
            self.staves
        )
    }

    /// Sets the tempo of the score in bpm
    pub fn tempo(mut self, tempo: usize) -> Self {
        self.tempo = tempo;
        self
    }

    pub fn signature(mut self, signature: TimeSignature) -> Self {
        self.signature = signature;
        self
    }

    pub fn name<S: AsRef<str>>(mut self, name: S) -> Self {
        self.name = name.as_ref().to_string();
        self
    }

    /// Extends the instrument pool of the score with the given instrument
    pub fn with_instrument(mut self, instrument: Box<dyn Instrument>) -> Self {
        self.instruments.push(instrument);
        self
    }

    /// Extends the instrument pool of the score with the given instruments.
    /// The order of the instruments remains the same, but gets offset by the
    /// instruments already present in the score's pool
    pub fn with_instruments(mut self, instruments: Vec<Box<dyn Instrument>>) -> Self {
        self.instruments.extend(instruments);
        self
    }
}