use std::sync::Arc;

use rustic_meta::MixMode;

use crate::core::audio::{Block, CHANNELS, silent_block};
use crate::core::graph::Filter;

/// A node in the audio graph. Wraps a [`Filter`], owns the per-port input
/// accumulator and the mix strategy, so [`System`] needs no global maps for
/// pending blocks or mix modes.
pub(super) struct AudioNode {
    pub(super) filter: Box<dyn Filter>,
    /// Per-port incoming blocks, accumulated between pushes and cleared after each process().
    inputs: Vec<Vec<Arc<Block>>>,
    mix_mode: MixMode,
}

impl AudioNode {
    pub(super) fn new(filter: Box<dyn Filter>, mix_mode: MixMode) -> Self {
        Self {
            filter,
            inputs: Vec::new(),
            mix_mode,
        }
    }

    /// Accumulate an incoming block on `port`.
    pub(super) fn push(&mut self, block: Arc<Block>, port: usize) {
        if port >= self.inputs.len() {
            self.inputs.resize_with(port + 1, Vec::new);
        }
        self.inputs[port].push(block);
    }

    /// Mix all accumulated inputs, run the inner filter, return Arc-wrapped outputs.
    /// Clears all input accumulators as a side effect.
    pub(super) fn process(&mut self, block_size: usize) -> Vec<Arc<Block>> {
        for (port, blocks) in self.inputs.iter_mut().enumerate() {
            if !blocks.is_empty() {
                let mixed = mix_blocks(std::mem::take(blocks), &self.mix_mode, block_size);
                self.filter.push(mixed, port);
            }
        }
        self.filter.transform().into_iter().map(Arc::new).collect()
    }

    pub(super) fn filter_mut(&mut self) -> &mut Box<dyn Filter> {
        &mut self.filter
    }

    pub(super) fn set_mix_mode(&mut self, mode: MixMode) {
        self.mix_mode = mode;
    }

    pub(super) fn mix_mode(&self) -> MixMode {
        self.mix_mode.clone()
    }

    pub(super) fn postponable(&self) -> bool {
        self.filter.postponable()
    }
}

impl Clone for AudioNode {
    fn clone(&self) -> Self {
        Self {
            filter: dyn_clone::clone_box(&*self.filter),
            inputs: self.inputs.clone(),
            mix_mode: self.mix_mode.clone(),
        }
    }
}

impl std::fmt::Debug for AudioNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AudioNode({:?})", self.filter)
    }
}

/// Reduce a collection of blocks arriving at the same port using the given mix strategy.
fn mix_blocks(blocks: Vec<Arc<Block>>, mode: &MixMode, block_size: usize) -> Arc<Block> {
    match blocks.len() {
        0 => Arc::new(silent_block(block_size)),
        1 => blocks.into_iter().next().unwrap(),
        _ => {
            let count = blocks.len();
            match mode {
                MixMode::Sum | MixMode::Average => {
                    let mut acc = silent_block(block_size);
                    for block in &blocks {
                        for (af, bf) in acc.iter_mut().zip(block.iter()) {
                            for ch in 0..CHANNELS {
                                af[ch] += bf[ch];
                            }
                        }
                    }
                    if matches!(mode, MixMode::Average) {
                        let inv = 1.0 / count as f32;
                        acc.iter_mut()
                            .for_each(|f| f.iter_mut().for_each(|s| *s *= inv));
                    }
                    Arc::new(acc)
                }
                MixMode::Max => {
                    let mut acc = (*blocks[0]).clone();
                    for block in &blocks[1..] {
                        for (af, bf) in acc.iter_mut().zip(block.iter()) {
                            for ch in 0..CHANNELS {
                                af[ch] = af[ch].max(bf[ch]);
                            }
                        }
                    }
                    Arc::new(acc)
                }
                MixMode::Min => {
                    let mut acc = (*blocks[0]).clone();
                    for block in &blocks[1..] {
                        for (af, bf) in acc.iter_mut().zip(block.iter()) {
                            for ch in 0..CHANNELS {
                                af[ch] = af[ch].min(bf[ch]);
                            }
                        }
                    }
                    Arc::new(acc)
                }
            }
        }
    }
}
