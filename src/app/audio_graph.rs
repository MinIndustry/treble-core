//! AudioGraph — assembles all instruments into a single compiled `System`.
//!
//! `AudioGraph::compile()` calls `instrument.into_system()` for each slot,
//! absorbs every sub-graph into one master graph, wires all outputs through a
//! single `GainFilter` node at port 0 (the run-loop mixes them via `MixMode::Sum`),
//! and returns the ready-to-run `System` for the render thread.

use std::collections::HashMap;

use crate::core::filters::prelude::{GainFilter, Limiter};
use crate::core::graph::{AudioGraphError, AudioOutputSink, System};
use crate::instruments::Instrument;

/// A single instrument slot inside the audio graph.
pub struct InstrumentSlot {
    /// The instrument that will be converted to a sub-graph on compile.
    pub instrument: Box<dyn Instrument>,
    /// Optional per-instrument filter chain (merged after `into_system()`).
    /// Currently unused — reserved for Phase 4.
    pub filters: Option<System>,
}

/// Manages all instruments and compiles them into a single `System`.
///
/// After calling `compile()`, `source_map` records which source index inside
/// the compiled `System` belongs to each instrument slot (by slot index).
/// Use those indices with `System::start_note()` / `System::stop_note()`.
#[derive(Default)]
pub struct AudioGraph {
    instruments: Vec<InstrumentSlot>,
    /// Maps slot index → first source index in the most recent compiled System.
    pub source_map: HashMap<usize, usize>,
}

impl AudioGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an instrument and return its slot index.
    pub fn add_instrument(&mut self, instrument: Box<dyn Instrument>) -> usize {
        let idx = self.instruments.len();
        self.instruments.push(InstrumentSlot {
            instrument,
            filters: None,
        });
        idx
    }

    /// Number of instrument slots.
    pub fn len(&self) -> usize {
        self.instruments.len()
    }

    pub fn is_empty(&self) -> bool {
        self.instruments.is_empty()
    }

    /// Consume all instrument slots and compile them into a unified `System`.
    ///
    /// The returned `System` has one `AudioOutputSink` and is ready to be
    /// swapped into the render thread. The `source_map` is updated so the
    /// caller can route `NoteStart`/`NoteStop` to the correct source index.
    ///
    /// The `instruments` vec is emptied by this call; re-add instruments if
    /// you need to compile again.
    pub fn compile(&mut self, sample_rate: f32) -> Result<System, AudioGraphError> {
        if self.instruments.is_empty() {
            return Ok(System::silent());
        }

        let slots: Vec<InstrumentSlot> = std::mem::take(&mut self.instruments);
        let n = slots.len();

        let mut main = System::new();
        let mut output_nodes = Vec::with_capacity(n);

        self.source_map.clear();

        for (slot_idx, slot) in slots.into_iter().enumerate() {
            let source_start = main.sources_len();

            let inst_system = slot.instrument.into_system(sample_rate);
            let output_node = main.absorb(inst_system)?;

            // Every absorbed source up to source_start is this instrument's
            let source_count = main.sources_len() - source_start;
            if source_count > 0 {
                self.source_map.insert(slot_idx, source_start);
            }

            output_nodes.push(output_node);
        }

        // Wire all instrument outputs into a passthrough gain node.
        // All connect on the same port 0; run() accumulates and sums them automatically.
        let mixer_node = main.add_filter(Box::new(GainFilter::new(1.0)));
        for &out_node in output_nodes.iter() {
            main.connect(out_node, mixer_node, 0, 0);
        }

        // Limiter prevents hard clipping when instruments sum above 1.0.
        let limiter_node = main.add_filter(Box::new(Limiter::default()));
        main.connect(mixer_node, limiter_node, 0, 0);

        // Final sink
        let sink = Box::new(AudioOutputSink::new());
        let sink_idx = main.add_sink(sink);
        main.connect_sink(limiter_node, sink_idx, 0);

        main.compute()?;

        Ok(main)
    }
}
