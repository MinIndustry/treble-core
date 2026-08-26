//! Sample-accurate note-event scheduling.
//!
//! The render thread used to apply every pending note message at the start of
//! the next block, quantizing note timing to block boundaries (up to ~11.6 ms
//! of jitter at 512 frames / 44.1 kHz). This module removes that quantization:
//! events carry an absolute engine frame, and [`render_block`] splits a block
//! into sub-block segments at exactly those frames.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use super::messages::InstrumentAudioMessage;
use crate::core::audio::{Block, silent_block};
use crate::core::graph::System;

enum ScheduledAction {
    Instrument(InstrumentAudioMessage),
    GraphSwap {
        system: System,
        fade_in_frames: u64,
        tail_frames: u64,
    },
}

struct GraphTransition {
    previous: System,
    elapsed_frames: u64,
    fade_in_frames: u64,
    tail_frames: u64,
}

/// A note event waiting for its frame. Ordered as a min-heap entry by
/// `(at_frame, seq)` — `seq` preserves submission order within a frame.
struct Entry {
    at_frame: u64,
    seq: u64,
    action: ScheduledAction,
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.at_frame == other.at_frame && self.seq == other.seq
    }
}
impl Eq for Entry {}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Entry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reversed: BinaryHeap is a max-heap, we want the earliest frame on top.
        (other.at_frame, other.seq).cmp(&(self.at_frame, self.seq))
    }
}

/// Min-heap of timestamped note events for the render thread.
#[derive(Default)]
pub struct EventScheduler {
    heap: BinaryHeap<Entry>,
    next_seq: u64,
    transition: Option<GraphTransition>,
}

impl EventScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue `command` to be applied at engine frame `at_frame`.
    /// Frames already in the past are applied at the next segment boundary.
    pub fn schedule(&mut self, at_frame: u64, command: InstrumentAudioMessage) {
        self.heap.push(Entry {
            at_frame,
            seq: self.next_seq,
            action: ScheduledAction::Instrument(command),
        });
        self.next_seq += 1;
    }

    /// Queue a graph generation change. Future events already present belong
    /// to the old generation and must not leak into the replacement graph.
    pub fn schedule_graph_swap(
        &mut self,
        at_frame: u64,
        system: System,
        fade_in_frames: u64,
        tail_frames: u64,
    ) {
        self.heap.retain(|entry| entry.at_frame < at_frame);
        self.heap.push(Entry {
            at_frame,
            seq: self.next_seq,
            action: ScheduledAction::GraphSwap {
                system,
                fade_in_frames,
                tail_frames,
            },
        });
        self.next_seq += 1;
    }

    /// Frame of the earliest pending event, if any.
    pub fn next_due(&self) -> Option<u64> {
        self.heap.peek().map(|e| e.at_frame)
    }

    /// Pop the earliest event if it is due at or before `frame`.
    /// Call in a loop to drain everything due.
    pub fn pop_due(&mut self, frame: u64) -> Option<InstrumentAudioMessage> {
        if self.heap.peek().is_none_or(|entry| entry.at_frame > frame) {
            return None;
        }
        if matches!(
            self.heap.peek().map(|entry| &entry.action),
            Some(ScheduledAction::Instrument(_))
        ) && let Some(entry) = self.heap.pop()
            && let ScheduledAction::Instrument(command) = entry.action
        {
            return Some(command);
        }
        None
    }

    fn pop_due_action(&mut self, frame: u64) -> Option<ScheduledAction> {
        self.heap
            .peek()
            .is_some_and(|entry| entry.at_frame <= frame)
            .then(|| self.heap.pop())
            .flatten()
            .map(|entry| entry.action)
    }

    pub fn len(&self) -> usize {
        self.heap.len()
    }

    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    /// Drop all pending events (e.g. when the graph is cleared).
    pub fn clear(&mut self) {
        self.heap.clear();
        self.transition = None;
    }
}

fn consume_frames(system: &mut System, frames: usize) -> Block {
    system.run_frames(frames);
    let Ok(sink) = system.get_sink(0) else {
        return silent_block(frames);
    };
    let mut block = sink.consume();
    block.resize(frames, [0.0; crate::core::audio::CHANNELS]);
    block
}

fn render_segment(
    system: &mut System,
    scheduler: &mut EventScheduler,
    frames: usize,
    out: &mut Vec<f32>,
) {
    let current = consume_frames(system, frames);
    let Some(transition) = scheduler.transition.as_mut() else {
        for frame in current {
            out.extend(frame);
        }
        return;
    };

    let previous = consume_frames(&mut transition.previous, frames);
    for (offset, (old, new)) in previous.iter().zip(&current).enumerate() {
        let age = transition.elapsed_frames + offset as u64;
        let new_gain = if transition.fade_in_frames == 0 {
            1.0
        } else {
            (age as f32 / transition.fade_in_frames as f32).clamp(0.0, 1.0)
        };
        let tail_progress = if transition.tail_frames == 0 {
            1.0
        } else {
            (age as f32 / transition.tail_frames as f32).clamp(0.0, 1.0)
        };
        let old_gain = (-6.0 * tail_progress).exp() * (1.0 - tail_progress);
        for channel in 0..crate::core::audio::CHANNELS {
            out.push(old[channel] * old_gain + new[channel] * new_gain);
        }
    }
    transition.elapsed_frames += frames as u64;
    if transition.elapsed_frames >= transition.tail_frames {
        scheduler.transition = None;
    }
}

/// Apply a note message to the system immediately.
pub(crate) fn apply_instrument_message(system: &mut System, command: InstrumentAudioMessage) {
    match command {
        InstrumentAudioMessage::NoteStart {
            source_index,
            note,
            velocity,
        } => system.start_note(source_index, note, velocity),
        InstrumentAudioMessage::NoteStop { source_index, note } => {
            system.stop_note(source_index, note)
        }
    }
}

/// Render one block of `system.block_size()` frames starting at engine frame
/// `start_frame`, splitting the render at every scheduled event so each event
/// applies on exactly its frame. Consumed sink frames are appended to `out`
/// as stereo-interleaved samples. Returns the engine frame after the block.
///
/// Events due at or before `start_frame` (late or immediate) apply before the
/// first sample; events due exactly at the block end apply at the start of
/// the next call.
pub fn render_block(
    system: &mut System,
    scheduler: &mut EventScheduler,
    start_frame: u64,
    out: &mut Vec<f32>,
) -> u64 {
    let block_end = start_frame + system.block_size() as u64;
    let mut current = start_frame;

    // Anchor time-based filters (LFOs) to the engine timeline, so a graph
    // hot-swap mid-performance resumes sweeps at the correct phase.
    system.broadcast_transport(start_frame);
    // Parameter ramps are evaluated once per block (design decision D3), at
    // the same engine frame the LFOs are anchored to.
    system.apply_automations(start_frame);

    while current < block_end {
        // Apply everything due now (including late events).
        while let Some(action) = scheduler.pop_due_action(current) {
            match action {
                ScheduledAction::Instrument(command) => {
                    apply_instrument_message(system, command);
                }
                ScheduledAction::GraphSwap {
                    system: new_system,
                    fade_in_frames,
                    tail_frames,
                } => {
                    let previous = std::mem::replace(system, new_system);
                    // The replacement's filters carry their build-time
                    // parameters; a sweep in flight must land before the new
                    // graph renders its first sample, or the swap is audible
                    // as the ramp jumping back to its declared start.
                    system.apply_automations(current);
                    scheduler.transition = Some(GraphTransition {
                        previous,
                        elapsed_frames: 0,
                        fade_in_frames,
                        tail_frames,
                    });
                }
            }
        }

        // Render up to the next event in this block, or the block end.
        let segment_end = scheduler
            .next_due()
            .filter(|&f| f < block_end)
            .map(|f| f.max(current + 1)) // guard against same-frame loop
            .unwrap_or(block_end);

        render_segment(system, scheduler, (segment_end - current) as usize, out);
        current = segment_end;
    }

    block_end
}
