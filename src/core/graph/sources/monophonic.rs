use serde::{Deserialize, Serialize};

use crate::{
    Note,
    core::{
        Block,
        audio::{mono_to_frame, silent_block},
        generator::prelude::MultiToneGenerator,
    },
};

use super::polyphonic::declick_decay;
use crate::core::graph::Source;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
/// Strategies for replacing or not a playing note in the monophonic generator.
pub enum MonophonicAllocationStrategy {
    #[default]
    Replace,
    Drop,
    // TODO: Add a conditional replacement based on current output power.
}

#[derive(Debug, Clone)]
/// A monophonic source for the graph system.
pub struct MonophonicSource {
    generator: MultiToneGenerator,
    replacement_strategy: MonophonicAllocationStrategy,
    sample_rate: f32,
    /// When false, `start_note()` triggers the generator without updating its frequency.
    /// Set to false for percussive instruments with fixed tuning (kick, snare, etc.).
    track_pitch: bool,
    active: bool,
    released: bool,
    current_note: Option<Note>,
    current_velocity: Option<f32>,
    /// The previous output sample, velocity included — what a replacement
    /// has to stay continuous with.
    last_out: f32,
    /// Set at a replacement to the value that was sounding. The next ticked
    /// sample measures the actual step against it, since a restarted
    /// generator does not necessarily restart near zero.
    declick_from: Option<f32>,
    /// Residual step from replacing a sounding note, decayed to zero over
    /// [`super::polyphonic::DECLICK_SECONDS`] so the handover does not click.
    declick: f32,
}

impl MonophonicSource {
    pub fn new(
        generator: MultiToneGenerator,
        sample_rate: f32,
        replacement_strategy: MonophonicAllocationStrategy,
    ) -> Self {
        Self {
            generator,
            replacement_strategy,
            sample_rate,
            track_pitch: true,
            active: false,
            released: false,
            current_note: None,
            current_velocity: None,
            last_out: 0.0,
            declick_from: None,
            declick: 0.0,
        }
    }

    /// Percussive sources do not track the pitch of the played notes
    /// but keep their original base_frequency
    pub fn new_percussive(
        generator: MultiToneGenerator,
        sample_rate: f32,
        replacement_strategy: MonophonicAllocationStrategy,
    ) -> Self {
        Self {
            track_pitch: false,
            ..Self::new(generator, sample_rate, replacement_strategy)
        }
    }

    fn should_replace(&self) -> bool {
        // TODO: Update with power based replacement strategy
        matches!(
            self.replacement_strategy,
            MonophonicAllocationStrategy::Replace
        )
    }
}

impl From<MultiToneGenerator> for MonophonicSource {
    fn from(generator: MultiToneGenerator) -> Self {
        Self::new(generator, 44100.0, MonophonicAllocationStrategy::Replace)
    }
}

impl Source for MonophonicSource {
    fn pull(&mut self, block_size: usize) -> Block {
        if !self.active {
            return silent_block(block_size);
        }

        let dt = 1.0 / self.sample_rate; // Delta time
        let decay = declick_decay(self.sample_rate);
        let samples = self.generator.tick_block(block_size, dt);
        if self.released && self.generator.completed() {
            self.active = false;
            self.released = false;
            self.last_out = 0.0;
            self.declick_from = None;
            self.declick = 0.0;
        }
        let velocity = self.current_velocity.unwrap_or(1.0);
        samples
            .into_iter()
            .map(|s| {
                let mut v = s * velocity;
                if let Some(previous) = self.declick_from.take() {
                    self.declick = previous - v;
                }
                if self.declick != 0.0 {
                    v += self.declick;
                    self.declick *= decay;
                    if self.declick.abs() < 1e-5 {
                        self.declick = 0.0;
                    }
                }
                self.last_out = v;
                mono_to_frame(v)
            })
            .collect()
    }

    fn start(&mut self) {
        if self.active {
            match self.replacement_strategy {
                MonophonicAllocationStrategy::Replace => {
                    // Replacing a sounding note restarts the envelope — a
                    // step in the output. Remember what was sounding so the
                    // next sample can measure and absorb the actual step.
                    self.declick_from = Some(self.last_out);
                    self.generator.start();
                    self.active = true;
                    self.released = false;
                }
                MonophonicAllocationStrategy::Drop => {}
            }
        } else {
            self.generator.start();
            self.active = true;
            self.released = false;
        }
    }

    fn stop(&mut self) {
        self.generator.stop();
        self.released = true;
    }

    fn kill(&mut self) {
        self.generator.stop();
        self.active = false;
        self.released = false;
        self.current_velocity = None;
        self.last_out = 0.0;
        self.declick_from = None;
        self.declick = 0.0;
    }

    fn start_note(&mut self, note: crate::Note, velocity: f32) {
        if self.should_replace() {
            self.current_note = Some(note);
            self.current_velocity = Some(velocity);
            if self.track_pitch {
                self.generator.set_base_frequency(note.frequency());
            }
            self.start();
        }
    }

    fn stop_note(&mut self, note: crate::Note) {
        if !self.track_pitch {
            self.stop();
            return;
        }
        if let Some(current_note) = self.current_note
            && current_note == note
        {
            self.stop();
        }
    }

    fn kill_note(&mut self, note: crate::Note) {
        if self.active && (!self.track_pitch || self.current_note == Some(note)) {
            self.generator.cutoff();
            self.released = true;
            self.current_note = None;
        }
    }

    fn is_active(&self) -> bool {
        self.active
    }
}

#[cfg(test)]
mod declick_tests {
    use super::*;
    use crate::core::generator::prelude::builder::{
        MultiToneGeneratorBuilder, ToneGeneratorBuilder,
    };
    use crate::core::generator::prelude::{FrequencyRelation, Waveform};

    const SAMPLE_RATE: f32 = 44100.0;

    fn sine_mono() -> MonophonicSource {
        let tone = ToneGeneratorBuilder::new()
            .waveform(Waveform::Sine)
            .frequency_relation(FrequencyRelation::Identity)
            .build();
        let generator = MultiToneGeneratorBuilder::new()
            .frequency(55.0)
            .add_generator(tone)
            .build();
        MonophonicSource::new(
            generator,
            SAMPLE_RATE,
            MonophonicAllocationStrategy::Replace,
        )
    }

    fn worst_step(blocks: &[Block]) -> f32 {
        let samples: Vec<f32> = blocks
            .iter()
            .flat_map(|block| block.iter().map(|frame| frame[0]))
            .collect();
        samples
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs())
            .fold(0.0, f32::max)
    }

    /// Replacing a sounding note must not step the output.
    ///
    /// It did: `replace` restarted the generator in the same sample, so the
    /// envelope fell from its sustained level to the start of the attack in
    /// one sample — an audible click on every overlapping bass note.
    #[test]
    fn replacing_a_sounding_note_does_not_click() {
        let mut mono = sine_mono();
        mono.start_note(Note::from_midi(33), 1.0);
        let mut blocks = vec![mono.pull(2048)];
        // Replace it mid-sound, with a different velocity for good measure.
        mono.start_note(Note::from_midi(40), 0.4);
        blocks.push(mono.pull(2048));
        let step = worst_step(&blocks);
        assert!(
            step < 0.05,
            "a mono replace stepped the output by {step} in one sample"
        );
    }

    /// A note started from silence carries no ramp: the fix must not smear
    /// clean attacks.
    #[test]
    fn a_fresh_note_gets_no_declick_ramp() {
        let mut mono = sine_mono();
        mono.start_note(Note::from_midi(33), 1.0);
        mono.pull(64);
        assert_eq!(mono.declick, 0.0, "a fresh start must not be compensated");
    }
}
