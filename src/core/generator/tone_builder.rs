use crate::core::envelope::Envelope;

pub struct ToneGeneratorBuilder {
    waveform: super::prelude::Waveform,
    freq_relation: Option<super::prelude::FrequencyRelation>,
    pitch_envelope: Option<Box<dyn Envelope>>,
    amplitude_envelope: Option<Box<dyn Envelope>>,
    current_frequency: f32,
}

impl Default for ToneGeneratorBuilder {
    fn default() -> Self {
        Self {
            waveform: super::prelude::Waveform::Sine,
            freq_relation: None,
            pitch_envelope: None,
            amplitude_envelope: None,
            current_frequency: 440.0,
        }
    }
}

impl ToneGeneratorBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn waveform(mut self, waveform: super::prelude::Waveform) -> Self {
        self.waveform = waveform;
        self
    }

    pub fn frequency_relation(
        mut self,
        relation: super::prelude::FrequencyRelation,
    ) -> Self {
        self.freq_relation = Some(relation);
        self
    }

    pub fn pitch_envelope(
        mut self,
        envelope: Option<Box<dyn Envelope>>,
    ) -> Self {
        self.pitch_envelope = envelope;
        self
    }

    pub fn amplitude_envelope(
        mut self,
        envelope: Option<Box<dyn Envelope>>,
    ) -> Self {
        self.amplitude_envelope = envelope;
        self
    }

    pub fn build(self) -> super::tone::ToneGenerator {
        super::tone::ToneGenerator::new(
            self.waveform,
            self.freq_relation,
            self.pitch_envelope,
            self.amplitude_envelope,
        )
    }
}
