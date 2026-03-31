use std::collections::HashMap;

use crate::Note;
use crate::core::envelope::prelude::{ADSREnvelopeBuilder, BezierSegment, LinearSegment};
use crate::core::envelope::{
    Envelope,
    prelude::{ADSREnvelope, ConstantSegment},
};
use crate::core::filters::prelude::GainFilter;
use crate::core::generator::prelude::{
    FrequencyRelation, MultiToneGenerator, Waveform,
    builder::{MultiToneGeneratorBuilder, ToneGeneratorBuilder},
};
use crate::core::graph::sources::{PolyphonicAllocationStrategy, PolyphonicSource};
use crate::core::graph::{SimpleSink, System};
use crate::core::utils::tones::TONES_FREQ;
use crate::instruments::Instrument;
use crate::instruments::voices::{PolyVoiceAllocator, PolyphonicVoice};

#[derive(Debug)]
pub struct Keyboard {
    generators: Vec<(MultiToneGenerator, bool)>,
    allocator: PolyVoiceAllocator,
    note_indices: HashMap<Note, usize>,
    output: f32,
}

impl PolyphonicVoice for Keyboard {
    fn with_voices(mut self, voices: usize) -> Self {
        self.generators = Vec::with_capacity(voices);
        self
    }

    fn with_allocator(mut self, allocator: PolyVoiceAllocator) -> Self {
        self.allocator = allocator;
        self
    }
}

impl Keyboard {
    fn piano_envelope(duration: f32) -> Box<dyn Envelope> {
        Box::new(
            ADSREnvelopeBuilder::new()
                .attack(Box::new(BezierSegment::new(0.0, 1.0, 0.1, (0.0, 1.0))))
                .decay(Box::new(BezierSegment::new(1.0, 0.0, duration, (0.0, 0.0))))
                .sustain(Box::new(ConstantSegment::new(0.0, None)))
                .release(Box::new(LinearSegment::new(0.2, 0.0, 0.3)))
                .build(),
        )
    }

    pub fn new(voices: usize, voice_allocator: PolyVoiceAllocator, envelope: ADSREnvelope) -> Self {
        let generators = std::iter::repeat_with(|| {
            // Per-generator envelopes are static mix ratios only.
            // The ADSR that controls note on/off lives on the MultiToneGenerator
            // so that ALL generators (sine + noise) fade together on key release,
            // and completed() can rely on a single authoritative envelope.
            let generator = MultiToneGeneratorBuilder::new()
                .add_generator(
                    ToneGeneratorBuilder::new()
                        .amplitude_envelope(Self::piano_envelope(5.0))
                        .waveform(Waveform::Sine)
                        .frequency_relation(FrequencyRelation::Identity)
                        .build(),
                )
                .add_generator(
                    ToneGeneratorBuilder::new()
                        .amplitude_envelope(Self::piano_envelope(4.0))
                        .waveform(Waveform::Sine)
                        .frequency_relation(FrequencyRelation::Harmonic(1))
                        .build(),
                )
                .add_generator(
                    ToneGeneratorBuilder::new()
                        .amplitude_envelope(Self::piano_envelope(2.0))
                        .waveform(Waveform::Sine)
                        .frequency_relation(FrequencyRelation::Harmonic(2))
                        .build(),
                )
                .add_generator(
                    ToneGeneratorBuilder::new()
                        .amplitude_envelope(Self::piano_envelope(1.0))
                        .waveform(Waveform::Sine)
                        .frequency_relation(FrequencyRelation::Harmonic(3))
                        .build(),
                )
                .amplitude_envelope(Some(Box::new(envelope.clone())))
                .build();

            (generator, false)
        })
        .take(voices)
        .collect();

        Self {
            generators,
            allocator: voice_allocator,
            note_indices: HashMap::new(),
            output: 0.0,
        }
    }

    pub fn get_current_notes(&self) -> Vec<Note> {
        self.note_indices.keys().cloned().collect()
    }
}

impl Instrument for Keyboard {
    /// Starts playing the given note
    fn start_note(&mut self, note: Note, _velocity: f32) {
        let generator_position = self
            .generators
            .iter()
            .position(|(_, is_playing)| !is_playing);
        if let Some(position) = generator_position {
            // If there is a free generator, we use it
            self.generators[position]
                .0
                .set_base_frequency(TONES_FREQ[note.0 as usize][note.1 as usize]);
            self.generators[position].0.start();
            self.generators[position].1 = true;
            self.note_indices.insert(note, position);
        }
    }

    /// Stops playing the given note
    fn stop_note(&mut self, note: Note) {
        if let Some(position) = self.note_indices.get(&note) {
            self.generators[*position].0.stop()
        }
    }

    /// Returns the current output of the instrument
    fn get_output(&mut self) -> f32 {
        self.output
    }

    /// Advances the instrument by one tick
    fn tick(&mut self) {
        // Stop completed generators
        for i in 0..self.generators.len() {
            if self.generators[i].1 {
                self.generators[i].1 = !self.generators[i].0.completed();
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
            / self.generators.len() as f32
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
        let output = system.add_filter(Box::new(GainFilter::new(1.0)));
        system.connect_source(source_idx, output, 0);
        let sink_idx = system.add_sink(Box::new(SimpleSink::new()));
        system.connect_sink(output, sink_idx, 0);
        system.compute().expect("Keyboard system compute failed");
        system
    }
}
