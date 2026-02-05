use super::segment::{ConstantSegment, LinearSegment, Segment};

#[derive(Debug, Clone)]
pub struct ADSREnvelopeBuilder {
    attack: Box<dyn Segment>,
    decay: Box<dyn Segment>,
    sustain: Box<dyn Segment>,
    release: Box<dyn Segment>,
}

impl Default for ADSREnvelopeBuilder {
    fn default() -> Self {
        Self {
            attack: Box::new(LinearSegment::default_attack()),
            decay: Box::new(LinearSegment::default_decay()),
            sustain: Box::new(ConstantSegment::default_sustain()),
            release: Box::new(LinearSegment::default_release()),
        }
    }
}

impl ADSREnvelopeBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(self) -> super::adsr::ADSREnvelope {
        super::adsr::ADSREnvelope {
            attack: self.attack,
            decay: self.decay,
            sustain: self.sustain,
            release: self.release,
        }
    }

    /// Sets the attack segment of the envelope.
    pub fn attack(mut self, segment: Box<dyn Segment>) -> Self {
        self.attack = segment;
        self
    }

    /// Sets the decay segment of the envelope.
    pub fn decay(mut self, segment: Box<dyn Segment>) -> Self {
        // TODO: Check decay is correct
        self.decay = segment;
        self
    }

    /// Sets the sustain segment of the envelope.
    pub fn sustain(mut self, segment: Box<dyn Segment>) -> Self {
        self.sustain = segment;
        self
    }

    /// Sets the release section of the envelope.
    pub fn release(mut self, segment: Box<dyn Segment>) -> Self {
        self.release = segment;
        self
    }
}
