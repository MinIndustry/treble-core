use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::{
    Note,
    core::{Block, audio::silent_block, generator::prelude::MultiToneGenerator},
};

use crate::core::graph::Source;

/// Time constant of the retrigger de-click ramp, in seconds.
///
/// When a sounding voice is stolen or retriggered, its envelope restarts from
/// zero, which is a step discontinuity in the output — an audible click. The
/// step is measured at the moment it happens and decayed to nothing over this
/// time constant instead of being emitted at full height. ~1.5 ms is below
/// the threshold where the correction itself becomes audible, and after five
/// time constants (~7.5 ms) the residual is under 1%.
pub(crate) const DECLICK_SECONDS: f32 = 0.0015;

/// The per-sample decay factor for a de-click ramp at `sample_rate`.
pub(crate) fn declick_decay(sample_rate: f32) -> f32 {
    (-1.0 / (DECLICK_SECONDS * sample_rate)).exp()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
/// Strategies for replacing or not a playing note in the polyphonic generator.
pub enum PolyphonicAllocationStrategy {
    #[default]
    ReplaceOldest,
    ReplaceYoungest,
    ReplaceLoudest,
    ReplaceQuietest,
    ReplaceRandom,
    Drop,
}

/// One voice of the pool: a generator plus the lifecycle state around it.
#[derive(Debug, Clone)]
struct Voice {
    generator: MultiToneGenerator,
    active: bool,
    released: bool,
    velocity: Option<f32>,
    /// The voice's previous output sample, velocity included — what a
    /// retrigger has to stay continuous with.
    last_out: f32,
    /// Set at a steal or retrigger to the value the voice was emitting. The
    /// next ticked sample measures the actual step against it, since a
    /// restarted generator does not necessarily restart near zero.
    declick_from: Option<f32>,
    /// Residual step from a steal or retrigger, decayed to zero over
    /// [`DECLICK_SECONDS`] so the replacement does not click.
    declick: f32,
}

impl Voice {
    fn new(generator: MultiToneGenerator) -> Self {
        Self {
            generator,
            active: false,
            released: false,
            velocity: None,
            last_out: 0.0,
            declick_from: None,
            declick: 0.0,
        }
    }

    /// Restart this voice on a new note, carrying whatever it was emitting
    /// into the de-click ramp so the handover is continuous.
    fn retrigger(&mut self, frequency: f32, velocity: f32) {
        if self.active {
            // `last_out` already contains any residual ramp, so this — not
            // accumulation — is what keeps repeated steals continuous.
            self.declick_from = Some(self.last_out);
        }
        self.generator.set_base_frequency(frequency);
        self.generator.start();
        self.active = true;
        self.released = false;
        self.velocity = Some(velocity);
    }
}

#[derive(Debug, Clone)]
/// A polyphonic source for the graph system.
pub struct PolyphonicSource {
    generator_template: MultiToneGenerator,
    voices: Vec<Voice>,
    max_voices: usize,
    replacement_strategy: PolyphonicAllocationStrategy,
    sample_rate: f32,
    // Map Note to voice index in the pool
    current_notes: HashMap<Note, usize>,
    notes_age: VecDeque<usize>, // Active voice indices, oldest first
}

impl PolyphonicSource {
    pub fn new(
        generator_template: MultiToneGenerator,
        max_voices: usize,
        sample_rate: f32,
        replacement_strategy: PolyphonicAllocationStrategy,
    ) -> Self {
        Self {
            generator_template,
            voices: Vec::new(),
            max_voices,
            replacement_strategy,
            sample_rate,
            current_notes: HashMap::new(),
            notes_age: VecDeque::new(),
        }
    }

    /// Find the index of the first inactive voice slot in the pool.
    fn find_free_slot(&self) -> Option<usize> {
        self.voices.iter().position(|voice| !voice.active)
    }

    /// Get the voice index to evict based on the replacement strategy.
    fn get_eviction_index(&self) -> Option<usize> {
        match self.replacement_strategy {
            PolyphonicAllocationStrategy::ReplaceOldest => self.notes_age.front().copied(),
            PolyphonicAllocationStrategy::ReplaceYoungest => self.notes_age.back().copied(),
            // TODO: implement amplitude-based and random strategies
            PolyphonicAllocationStrategy::ReplaceLoudest
            | PolyphonicAllocationStrategy::ReplaceQuietest
            | PolyphonicAllocationStrategy::ReplaceRandom
            | PolyphonicAllocationStrategy::Drop => None,
        }
    }
}

impl From<MultiToneGenerator> for PolyphonicSource {
    fn from(generator: MultiToneGenerator) -> Self {
        Self::new(
            generator,
            8,
            44100.0,
            PolyphonicAllocationStrategy::default(),
        )
    }
}

impl Source for PolyphonicSource {
    fn pull(&mut self, block_size: usize) -> Block {
        if !self.voices.iter().any(|voice| voice.active) {
            return silent_block(block_size);
        }

        let dt = 1.0 / self.sample_rate;
        let decay = declick_decay(self.sample_rate);
        let mut out = silent_block(block_size);

        for voice in self.voices.iter_mut() {
            if !voice.active {
                continue;
            }

            if voice.generator.completed() && voice.released {
                voice.active = false;
                voice.released = false;
                voice.last_out = 0.0;
                voice.declick_from = None;
                voice.declick = 0.0;
                continue;
            }

            for frame in out.iter_mut() {
                let mut s = voice.generator.tick(dt) * voice.velocity.unwrap_or(1.0);
                if let Some(previous) = voice.declick_from.take() {
                    voice.declick = previous - s;
                }
                if voice.declick != 0.0 {
                    s += voice.declick;
                    voice.declick *= decay;
                    if voice.declick.abs() < 1e-5 {
                        voice.declick = 0.0;
                    }
                }
                voice.last_out = s;
                frame[0] += s;
                frame[1] += s;
            }
        }

        // Clean up tracking for voices that completed their release phase
        self.notes_age.retain(|&i| self.voices[i].active);
        self.current_notes.retain(|_, v| self.voices[*v].active);

        out
    }

    fn stop(&mut self) {
        for voice in self.voices.iter_mut() {
            voice.generator.stop();
            voice.released = true;
        }
        self.current_notes.clear();
    }

    fn kill(&mut self) {
        for voice in self.voices.iter_mut() {
            voice.generator.stop();
            voice.active = false;
            voice.released = false;
            voice.velocity = None;
            voice.last_out = 0.0;
            voice.declick_from = None;
            voice.declick = 0.0;
        }
        self.current_notes.clear();
        self.notes_age.clear();
    }

    fn start_note(&mut self, note: Note, velocity: f32) {
        let freq = note.frequency();

        // If the note is already held, retrigger in place
        if let Some(&index) = self.current_notes.get(&note) {
            self.voices[index].retrigger(freq, velocity);
            // Move to back of age queue (it is now the youngest)
            self.notes_age.retain(|&i| i != index);
            self.notes_age.push_back(index);
            return;
        }

        // Find or allocate a voice slot
        let index = if let Some(free) = self.find_free_slot() {
            free
        } else if self.voices.len() < self.max_voices {
            // Grow the pool up to max_voices. The new voice is told which one
            // it is, so its noise decorrelates from its siblings' without
            // depending on how many generators the process built before it.
            let mut generator = self.generator_template.clone();
            generator.set_voice(self.voices.len() as u64);
            self.voices.push(Voice::new(generator));
            self.voices.len() - 1
        } else {
            // Pool is full — apply replacement strategy
            match self.get_eviction_index() {
                None => return, // Drop: discard the new note
                Some(evicted) => {
                    self.current_notes.retain(|_, v| *v != evicted);
                    self.notes_age.retain(|&i| i != evicted);
                    evicted
                }
            }
        };

        self.voices[index].retrigger(freq, velocity);
        self.current_notes.insert(note, index);
        self.notes_age.push_back(index);
    }

    fn stop_note(&mut self, note: Note) {
        if let Some(&index) = self.current_notes.get(&note) {
            let voice = &mut self.voices[index];
            voice.generator.stop();
            voice.released = true;
            self.current_notes.remove(&note);
            // Keep in notes_age until the release phase finishes in pull()
        }
    }

    fn kill_note(&mut self, note: Note) {
        if let Some(index) = self.current_notes.remove(&note) {
            let voice = &mut self.voices[index];
            voice.generator.cutoff();
            voice.released = true;
        }
    }

    fn is_active(&self) -> bool {
        self.voices.iter().any(|voice| voice.active)
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

    /// A one-sine voice with the default ADSR — enough to be mid-note loud.
    fn sine_pool(max_voices: usize) -> PolyphonicSource {
        let tone = ToneGeneratorBuilder::new()
            .waveform(Waveform::Sine)
            .frequency_relation(FrequencyRelation::Identity)
            .build();
        let generator = MultiToneGeneratorBuilder::new()
            .frequency(55.0)
            .add_generator(tone)
            .build();
        PolyphonicSource::new(
            generator,
            max_voices,
            SAMPLE_RATE,
            PolyphonicAllocationStrategy::ReplaceOldest,
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

    /// Stealing a sounding voice must not step the output.
    ///
    /// It did: eviction stopped the old generator and restarted it in the
    /// same sample, so the envelope fell from its sustained level to the
    /// start of the attack in one sample — an audible click on every steal.
    #[test]
    fn a_steal_does_not_click() {
        let mut pool = sine_pool(1);
        pool.start_note(Note::from_midi(33), 1.0);
        // Let the one voice reach a loud part of its envelope.
        let mut blocks = vec![pool.pull(2048)];
        // Steal it for a new pitch mid-sound.
        pool.start_note(Note::from_midi(40), 1.0);
        blocks.push(pool.pull(2048));
        let step = worst_step(&blocks);
        assert!(
            step < 0.05,
            "a voice steal stepped the output by {step} in one sample"
        );
    }

    /// Retriggering a note that is still held is the same discontinuity by
    /// another path, and must be equally smooth.
    #[test]
    fn a_held_note_retrigger_does_not_click() {
        let mut pool = sine_pool(4);
        pool.start_note(Note::from_midi(33), 1.0);
        let mut blocks = vec![pool.pull(2048)];
        pool.start_note(Note::from_midi(33), 0.3);
        blocks.push(pool.pull(2048));
        let step = worst_step(&blocks);
        assert!(
            step < 0.05,
            "a held-note retrigger stepped the output by {step} in one sample"
        );
    }

    /// The compensation must actually die out: a stolen voice's ramp decays
    /// below audibility within a few milliseconds rather than lingering.
    #[test]
    fn the_declick_ramp_decays() {
        let mut pool = sine_pool(1);
        pool.start_note(Note::from_midi(33), 1.0);
        pool.pull(2048);
        pool.start_note(Note::from_midi(40), 1.0);
        pool.pull(1024); // ~23 ms — over fifteen time constants
        assert_eq!(
            pool.voices[0].declick, 0.0,
            "the de-click residual never reached zero"
        );
    }
}
