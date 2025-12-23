use crate::core::envelope::Envelope;

pub struct ToneGenerator {
    waveform: super::prelude::Waveform,
    frequency_relation: Option<super::prelude::FrequencyRelation>,
    pitch_envelope: Option<Box<dyn Envelope>>,
    amplitude_envelope: Option<Box<dyn Envelope>>,
    phase: f32,
    normalized_time: f32,
    is_stopped: bool,
    current_frequency: f32,
}

impl ToneGenerator {
    pub fn new(
        waveform: super::prelude::Waveform,
        frequency_relation: Option<super::prelude::FrequencyRelation>,
        pitch_envelope: Option<Box<dyn Envelope>>,
        amplitude_envelope: Option<Box<dyn Envelope>>,
    ) -> Self {
        Self {
            waveform,
            frequency_relation,
            pitch_envelope,
            amplitude_envelope,
            phase: 0.0,
            normalized_time: 0.0,
            is_stopped: true,
            current_frequency: 440.0,
        }
    }

    pub fn has_frequency_relation(&self) -> bool {
        self.frequency_relation.is_some()
    }
}