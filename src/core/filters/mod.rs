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

mod amplifier;
mod clipper;
mod combinator;
mod delay;
mod moving_average;
pub mod pass;
mod resonant_bandpass;
mod structural;
mod tremolo;

pub mod prelude {
    pub use super::amplifier::*;
    pub use super::clipper::*;
    pub use super::combinator::*;
    pub use super::delay::*;
    pub use super::moving_average::*;
    pub use super::pass::*;
    pub use super::resonant_bandpass::*;
    pub use super::structural::*;
    pub use super::tremolo::*;
}
