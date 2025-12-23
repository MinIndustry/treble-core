pub struct CompositeGenerator {
    tone_generators: Vec<super::prelude::ToneGenerator>,
    base_frequency: f32,
    mix_mode: super::prelude::MixMode,
    global_pitch_envelope: Option<Box<dyn Envelope>>,
    global_amplitude_envelope: Option<Box<dyn Envelope>>,
    normalized_time: f32,
    is_stopped: bool,
}