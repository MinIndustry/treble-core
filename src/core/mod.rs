/// Defines the different envelope shapes & types
/// Envelopes implement the `Envelope` trait and can
/// be of 3 types; linear, bezier, adsr
pub mod envelope;

/// Filters are structures that implement the `Filter` trait. They
/// operate on audio signals to modify their frequency response.
/// examples include low-pass, high-pass, tremolo, ...
pub mod filters;

/// Generators are structures that implement the `Generator` trait.
/// They generate audio signals of different types such as sine, square, sawtooth, etc.
pub mod generator;

/// Graphs are used to build audio pipelines for the application.
/// They serve as metastructures for filters.
/// Graphs can be used to create complex audio effects and processing chains.
pub mod graph;

/// The keys module provides mapping between keyboard input and application events.
/// It can be used to handle user input and trigger actions within the application.
pub mod keys;

/// Macros are used to define common patterns and structures within the application.
/// They can be used to simplify code and improve readability.
pub mod macros;
pub mod note;
pub mod score;

/// The tones module contains the mapping between musical notes and their corresponding frequencies.
/// It can be used to generate audio signals for different musical notes.
pub mod tones;
