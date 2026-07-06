use serde::{Deserialize, Serialize};

use crate::instruments::spec::InstrumentSpec;

/// Commands managing the instrument registry and instrument slots.
///
/// Registry mutations (`Register`/`Unregister`) are definition-time only:
/// they validate and store specs but do not touch the audio graph.
/// `Instantiate` compiles a registered spec into an instrument slot and
/// hot-swaps the running graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum InstrumentCommand {
    /// Validate a spec and add it to the registry (replaces same-named spec).
    /// Slots already instantiated from a previous version keep playing.
    Register(InstrumentSpec),
    /// Remove a spec from the registry. Instantiated slots are unaffected.
    Unregister { name: String },
    /// Add the named registered spec to the audio graph as a new instrument
    /// slot and, if the engine is running, recompile + hot-swap.
    /// A no-op when the name is already instantiated.
    Instantiate { name: String },
}
