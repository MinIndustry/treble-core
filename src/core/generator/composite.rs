use crate::core::envelope::Envelope;

pub struct CompositeGenerator {
    base_frequency: f32,
    tone_generators: Vec<super::prelude::ToneGenerator>,
    mix_mode: super::prelude::MixMode,
    global_pitch_envelope: Option<Box<dyn Envelope>>,
    global_amplitude_envelope: Option<Box<dyn Envelope>>,
    normalized_time: f32,
    is_stopped: bool,
}

impl CompositeGenerator {
    pub fn new(
        base_frequency: f32,
        mut tone_generators: Vec<super::prelude::ToneGenerator>,
        mix_mode: super::prelude::MixMode,
        global_pitch_envelope: Option<Box<dyn Envelope>>,
        global_amplitude_envelope: Option<Box<dyn Envelope>>,
    ) -> Self {
        // Setup individual tone generators' frequencies
        tone_generators.iter_mut().for_each(|tg| {
            if !tg.has_frequency_relation() {
                log::warn!("Adding a tone generator without a frequency relation to a composite generator. The generator will not get updated");
            } else {
                log::info!("Here we need to set the tone generator frequency from the base freq (using relations)");
            }
        });

        Self {
            base_frequency,
            tone_generators,
            mix_mode,
            global_pitch_envelope,
            global_amplitude_envelope,
            normalized_time: 0.0,
            is_stopped: true,
        }
    }
}