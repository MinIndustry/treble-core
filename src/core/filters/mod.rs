//! Filters — signal processors for frequency and time-domain shaping
//!
//! ## Overview
//! Filters operate on audio signals to change their frequency response, amplitude,
//! or temporal characteristics. This module contains implementations such as
//! gain/amplifiers, clippers, tremolo (amplitude LFO), delay lines, moving
//! averages and resonant band-pass filters.
//!
//! ## Usage notes
//! - Use small buffer sizes for low latency (see `start_app` usage in `lib.rs`).
//! - Prefer combining filters via `combinator` or the `graph` utilities for
//!   modular pipelines.

pub mod dynamics;
pub mod frequency;
pub mod modulation;
pub mod utility;

pub mod prelude {
    pub use super::dynamics::*;
    pub use super::frequency::*;
    pub use super::modulation::*;
    pub use super::utility::*;
}
