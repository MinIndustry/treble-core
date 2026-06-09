//! Configurable polyphonic synthesiser for live-coding instruments.

use std::collections::HashMap;

use crate::Note;
use crate::core::envelope::prelude::{
    ADSREnvelope, ADSREnvelopeBuilder, ConstantSegment, LinearSegment,
};
use crate::core::filters::prelude::{GainFilter, LowPassFilter};
use crate::core::generator::prelude::{
    FrequencyRelation, MultiToneGenerator, Waveform,
    builder::{MultiToneGeneratorBuilder, ToneGeneratorBuilder},
};
use crate::core::graph::sources::{PolyphonicAllocationStrategy, PolyphonicSource};
use crate::core::graph::{SimpleSink, System};
use crate::core::utils::tones::TONES_FREQ;
use crate::instruments::Instrument;
use crate::instruments::voices::{PolyVoiceAllocator, PolyphonicVoice};

/// Configuration for a [`Synth`] voice template.
#[derive(Debug, Clone)]
pub struct SynthConfig {
    pub waveform: Waveform,
    pub voices: usize,
    pub envelope: ADSREnvelope,
    pub lowpass_cutoff: Option<f32>,
    pub gain: f32,
}

impl SynthConfig {
    pub fn sine() -> Self {
        Self {
            waveform: Waveform::Sine,
            voices: 8,
            envelope: adsr(0.01, 0.1, 0.8, 0.2),
            lowpass_cutoff: None,
            gain: 0.8,
        }
    }

    pub fn saw() -> Self {
        Self {
            waveform: Waveform::Sawtooth,
            voices: 8,
            envelope: adsr(0.02, 0.15, 0.6, 0.15),
            lowpass_cutoff: Some(4000.0),
            gain: 0.7,
        }
    }

    pub fn square() -> Self {
        Self {
            waveform: Waveform::Square,
            voices: 8,
            envelope: adsr(0.01, 0.1, 0.5, 0.1),
            lowpass_cutoff: Some(3000.0),
            gain: 0.6,
        }
    }

    pub fn triangle() -> Self {
        Self {
            waveform: Waveform::Triangle,
            voices: 8,
            envelope: adsr(0.02, 0.2, 0.7, 0.25),
            lowpass_cutoff: None,
            gain: 0.75,
        }
    }

    pub fn piano() -> Self {
        Self {
            waveform: Waveform::Sine,
            voices: 8,
            envelope: adsr(0.005, 0.25, 0.4, 0.35),
            lowpass_cutoff: Some(3500.0),
            gain: 0.85,
        }
    }

    pub fn bass() -> Self {
        Self {
            waveform: Waveform::Square,
            voices: 4,
            envelope: adsr(0.005, 0.08, 0.7, 0.1),
            lowpass_cutoff: Some(800.0),
            gain: 0.9,
        }
    }

    pub fn pad() -> Self {
        Self {
            waveform: Waveform::Sawtooth,
            voices: 6,
            envelope: adsr(0.4, 0.6, 0.75, 1.2),
            lowpass_cutoff: Some(1200.0),
            gain: 0.55,
        }
    }

    pub fn pluck() -> Self {
        Self {
            waveform: Waveform::Triangle,
            voices: 6,
            envelope: adsr(0.001, 0.08, 0.05, 0.15),
            lowpass_cutoff: Some(5000.0),
            gain: 0.8,
        }
    }

    pub fn bell() -> Self {
        Self {
            waveform: Waveform::Sine,
            voices: 4,
            envelope: adsr(0.001, 0.6, 0.0, 0.8),
            lowpass_cutoff: Some(8000.0),
            gain: 0.7,
        }
    }
}

fn adsr(a: f32, d: f32, s: f32, r: f32) -> ADSREnvelope {
    ADSREnvelopeBuilder::new()
        .attack(Box::new(LinearSegment::new(0.0, 1.0, a)))
        .decay(Box::new(LinearSegment::new(1.0, s, d)))
        .sustain(Box::new(ConstantSegment::new(s, None)))
        .release(Box::new(LinearSegment::new(s, 0.0, r)))
        .build()
}

/// Polyphonic synthesiser with a configurable oscillator and optional low-pass.
#[derive(Debug)]
pub struct Synth {
    config: SynthConfig,
    generators: Vec<(MultiToneGenerator, bool)>,
    allocator: PolyVoiceAllocator,
    note_indices: HashMap<Note, usize>,
    output: f32,
}

impl PolyphonicVoice for Synth {
    fn with_voices(mut self, voices: usize) -> Self {
        self.config.voices = voices;
        self
    }

    fn with_allocator(mut self, allocator: PolyVoiceAllocator) -> Self {
        self.allocator = allocator;
        self
    }
}

impl Synth {
    pub fn new(config: SynthConfig) -> Self {
        let voices = config.voices.max(1);
        let envelope = config.envelope.clone();
        let waveform = config.waveform.clone();

        let generators = std::iter::repeat_with(|| {
            let generator = MultiToneGeneratorBuilder::new()
                .add_generator(
                    ToneGeneratorBuilder::new()
                        .waveform(waveform.clone())
                        .frequency_relation(FrequencyRelation::Identity)
                        .build(),
                )
                .amplitude_envelope(Some(Box::new(envelope.clone())))
                .build();
            (generator, false)
        })
        .take(voices)
        .collect();

        Self {
            config,
            generators,
            allocator: PolyVoiceAllocator::default(),
            note_indices: HashMap::new(),
            output: 0.0,
        }
    }

    pub fn from_name(name: &str) -> Self {
        let config = match name {
            "sine" => SynthConfig::sine(),
            "saw" => SynthConfig::saw(),
            "square" => SynthConfig::square(),
            "triangle" => SynthConfig::triangle(),
            "piano" => SynthConfig::piano(),
            "bass" => SynthConfig::bass(),
            "pad" => SynthConfig::pad(),
            "pluck" => SynthConfig::pluck(),
            "bell" => SynthConfig::bell(),
            _ => SynthConfig::piano(),
        };
        Self::new(config)
    }
}

impl Instrument for Synth {
    fn start_note(&mut self, note: Note, _velocity: f32) {
        if let Some(position) = self.generators.iter().position(|(_, playing)| !playing) {
            self.generators[position]
                .0
                .set_base_frequency(TONES_FREQ[note.0 as usize][note.1 as usize]);
            self.generators[position].0.start();
            self.generators[position].1 = true;
            self.note_indices.insert(note, position);
        }
    }

    fn stop_note(&mut self, note: Note) {
        if let Some(position) = self.note_indices.get(&note) {
            self.generators[*position].0.stop();
        }
    }

    fn get_output(&mut self) -> f32 {
        self.output
    }

    fn tick(&mut self) {
        for (generator, playing) in &mut self.generators {
            if *playing {
                *playing = !generator.completed();
            }
        }

        self.output = self
            .generators
            .iter_mut()
            .map(|(generator, is_playing)| {
                if *is_playing {
                    generator.tick(1.0 / 44100.0)
                } else {
                    0.0
                }
            })
            .sum::<f32>()
            / self.generators.len().max(1) as f32;
    }

    fn into_system(self: Box<Self>, sample_rate: f32) -> System {
        let voice_count = self.generators.len();
        let template = self
            .generators
            .into_iter()
            .next()
            .map(|(g, _)| g)
            .unwrap_or_default();

        let source = PolyphonicSource::new(
            template,
            voice_count.max(1),
            sample_rate,
            PolyphonicAllocationStrategy::default(),
        );

        let mut system = System::new();
        let source_idx = system.add_source(Box::new(source));
        let gain = system.add_filter(Box::new(GainFilter::new(self.config.gain)));

        if let Some(cutoff) = self.config.lowpass_cutoff {
            let lp = system.add_filter(Box::new(LowPassFilter::new(cutoff, sample_rate)));
            system.connect_source(source_idx, lp, 0);
            system.connect(lp, gain, 0, 0);
        } else {
            system.connect_source(source_idx, gain, 0);
        }

        let sink_idx = system.add_sink(Box::new(SimpleSink::new()));
        system.connect_sink(gain, sink_idx, 0);
        system.compute().expect("Synth system compute failed");
        system
    }
}
