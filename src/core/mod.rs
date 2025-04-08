/// Defines the different envelope shapes & types
/// Envelopes implement the `Envelope` trait and can
/// be of 3 types; linear, bezier, adsr
pub mod envelope;

pub mod filters;
pub mod generator;
pub mod graph;
pub mod keys;
pub mod macros;
pub mod note;
pub mod score;
pub mod tones;
