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
use crate::core::graph::System;

enum ScheduledAction {
    Instrument(InstrumentAudioMessage),
    GraphSwap(System),
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
    pub fn schedule_graph_swap(&mut self, at_frame: u64, system: System) {
        self.heap.retain(|entry| entry.at_frame < at_frame);
        self.heap.push(Entry {
            at_frame,
            seq: self.next_seq,
            action: ScheduledAction::GraphSwap(system),
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
        ) {
            let entry = self.heap.pop().expect("peeked scheduler entry must exist");
            if let ScheduledAction::Instrument(command) = entry.action {
                return Some(command);
            }
        }
        None
    }

    fn pop_due_action(&mut self, frame: u64) -> Option<ScheduledAction> {
        self.heap
            .peek()
            .is_some_and(|entry| entry.at_frame <= frame)
            .then(|| {
                self.heap
                    .pop()
                    .expect("peeked scheduler entry must exist")
                    .action
            })
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

    while current < block_end {
        // Apply everything due now (including late events).
        while let Some(action) = scheduler.pop_due_action(current) {
            match action {
                ScheduledAction::Instrument(command) => {
                    apply_instrument_message(system, command);
                }
                ScheduledAction::GraphSwap(new_system) => *system = new_system,
            }
        }

        // Render up to the next event in this block, or the block end.
        let segment_end = scheduler
            .next_due()
            .filter(|&f| f < block_end)
            .map(|f| f.max(current + 1)) // guard against same-frame loop
            .unwrap_or(block_end);

        system.run_frames((segment_end - current) as usize);
        if let Ok(sink) = system.get_sink(0) {
            for frame in sink.consume() {
                out.push(frame[0]);
                out.push(frame[1]);
            }
        }
        current = segment_end;
    }

    block_end
}
