use std::collections::{HashMap, VecDeque};

use crate::{
    Note,
    core::{
        Block,
        audio::{mono_to_frame, silent_block},
        generator::prelude::MultiToneGenerator,
    },
};

use crate::core::graph::Source;

#[derive(Debug, Clone, Default)]
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

#[derive(Debug, Clone)]
/// A polyphonic source for the graph system.
pub struct PolyphonicSource {
    generator_template: MultiToneGenerator,
    // Generator, active, released
    generators: Vec<(MultiToneGenerator, bool, bool)>,
    max_voices: usize,
    replacement_strategy: PolyphonicAllocationStrategy,
    sample_rate: f32,
    // Map Note to generator index in the pool
    current_notes: HashMap<Note, usize>,
    notes_age: VecDeque<usize>, // Active generator indices, oldest first
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
            generators: Vec::new(),
            max_voices,
            replacement_strategy,
            sample_rate,
            current_notes: HashMap::new(),
            notes_age: VecDeque::new(),
        }
    }

    /// Find the index of the first inactive generator slot in the pool.
    fn find_free_slot(&self) -> Option<usize> {
        self.generators.iter().position(|(_, active, _)| !*active)
    }

    /// Get the generator index to evict based on the replacement strategy.
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
        let any_active = self.generators.iter().any(|(_, a, _)| *a);

        if !any_active {
            return silent_block(block_size);
        }

        let dt = 1.0 / self.sample_rate;
        let samples: Vec<f32> = self
            .generators
            .iter_mut()
            .map(|(g, active, released)| {
                if !*active {
                    return vec![0.0; block_size];
                }
                if g.completed() && *released {
                    *active = false;
                    *released = false;
                    return vec![0.0; block_size];
                }
                g.tick_block(block_size, dt)
            })
            .fold(vec![0.0; block_size], |b1, b2| {
                b1.into_iter().zip(b2).map(|(a, b)| a + b).collect()
            });

        // Clean up tracking for generators that completed their release phase
        self.notes_age.retain(|&i| self.generators[i].1);
        self.current_notes.retain(|_, v| self.generators[*v].1);

        samples.into_iter().map(mono_to_frame).collect()
    }

    fn stop(&mut self) {
        for (g, _, released) in self.generators.iter_mut() {
            g.stop();
            *released = true;
        }
        self.current_notes.clear();
    }

    fn kill(&mut self) {
        for (g, active, released) in self.generators.iter_mut() {
            g.stop();
            *active = false;
            *released = false;
        }
        self.current_notes.clear();
        self.notes_age.clear();
    }

    fn start_note(&mut self, note: Note, _velocity: f32) {
        let freq = note.frequency();

        // If the note is already held, retrigger in place
        if let Some(&gen_index) = self.current_notes.get(&note) {
            let (g, active, released) = &mut self.generators[gen_index];
            g.set_base_frequency(freq);
            g.start();
            *active = true;
            *released = false;
            // Move to back of age queue (it is now the youngest)
            self.notes_age.retain(|&i| i != gen_index);
            self.notes_age.push_back(gen_index);
            return;
        }

        // Find or allocate a generator slot
        let gen_index = if let Some(free) = self.find_free_slot() {
            free
        } else if self.generators.len() < self.max_voices {
            // Grow the pool up to max_voices
            self.generators
                .push((self.generator_template.clone(), false, false));
            self.generators.len() - 1
        } else {
            // Pool is full — apply replacement strategy
            match self.get_eviction_index() {
                None => return, // Drop: discard the new note
                Some(evict_idx) => {
                    let (g, active, released) = &mut self.generators[evict_idx];
                    g.stop();
                    *active = false;
                    *released = false;
                    self.current_notes.retain(|_, v| *v != evict_idx);
                    self.notes_age.retain(|&i| i != evict_idx);
                    evict_idx
                }
            }
        };

        let (g, active, released) = &mut self.generators[gen_index];
        g.set_base_frequency(freq);
        g.start();
        *active = true;
        *released = false;

        self.current_notes.insert(note, gen_index);
        self.notes_age.push_back(gen_index);
    }

    fn stop_note(&mut self, note: Note) {
        if let Some(&gen_index) = self.current_notes.get(&note) {
            let (g, _, released) = &mut self.generators[gen_index];
            g.stop();
            *released = true;
            self.current_notes.remove(&note);
            // Keep in notes_age until the release phase finishes in pull()
        }
    }

    fn is_active(&self) -> bool {
        self.generators.iter().any(|(_, active, _)| *active)
    }
}
