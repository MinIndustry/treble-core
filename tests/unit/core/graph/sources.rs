//! Velocity → amplitude regression tests (BUG-003 / decision D5).
//!
//! The exact velocity curve is intentionally unspecified for now (linear today,
//! possibly perceptual later). These tests only pin curve-independent invariants:
//! - lower velocity ⇒ lower loudness (monotonicity),
//! - velocity 1.0 keeps the pre-velocity loudness (identity at full scale),
//! - velocity 0.0 is near-silent,
//! - retriggering a held note applies the new velocity.

use treble::Note;
use treble::core::Block;
use treble::core::envelope::prelude::ConstantSegment;
use treble::core::generator::prelude::{
    FrequencyRelation, MultiToneGenerator, Waveform,
    builder::{MultiToneGeneratorBuilder, ToneGeneratorBuilder},
};
use treble::core::graph::{
    MonophonicAllocationStrategy, MonophonicSource, PolyphonicAllocationStrategy, PolyphonicSource,
    Source,
};

const SAMPLE_RATE: f32 = 44100.0;
const BLOCK_SIZE: usize = 4096;

/// Steady 440 Hz sine with a constant unit envelope — RMS ≈ 1/√2 while playing,
/// independent of the oscillator's randomized initial phase.
fn steady_sine() -> MultiToneGenerator {
    MultiToneGeneratorBuilder::new()
        .add_generator(
            ToneGeneratorBuilder::new()
                .waveform(Waveform::Sine)
                .frequency_relation(FrequencyRelation::Identity)
                .amplitude_envelope(Box::new(ConstantSegment::new(1.0, None)))
                .build(),
        )
        .frequency(440.0)
        .build()
}

fn rms(block: &Block) -> f32 {
    let sum: f32 = block.iter().map(|f| f[0] * f[0]).sum();
    (sum / block.len() as f32).sqrt()
}

fn mono_rms_at_velocity(velocity: f32) -> f32 {
    let mut source = MonophonicSource::new(
        steady_sine(),
        SAMPLE_RATE,
        MonophonicAllocationStrategy::Replace,
    );
    source.start_note(Note::from_midi(69), velocity);
    rms(&source.pull(BLOCK_SIZE))
}

fn poly_rms_at_velocity(velocity: f32) -> f32 {
    let mut source = PolyphonicSource::new(
        steady_sine(),
        4,
        SAMPLE_RATE,
        PolyphonicAllocationStrategy::default(),
    );
    source.start_note(Note::from_midi(69), velocity);
    rms(&source.pull(BLOCK_SIZE))
}

#[test]
fn monophonic_lower_velocity_is_quieter() {
    let loud = mono_rms_at_velocity(1.0);
    let mid = mono_rms_at_velocity(0.5);
    let quiet = mono_rms_at_velocity(0.25);
    assert!(
        loud > 0.1,
        "full-velocity note should produce signal (rms = {loud})"
    );
    assert!(
        mid < loud * 0.95,
        "velocity 0.5 (rms {mid}) should be quieter than 1.0 (rms {loud})"
    );
    assert!(
        quiet < mid * 0.95,
        "velocity 0.25 (rms {quiet}) should be quieter than 0.5 (rms {mid})"
    );
}

#[test]
fn monophonic_full_velocity_keeps_unit_loudness() {
    // Whatever curve is chosen later, velocity 1.0 must map to gain 1.0 so that
    // existing callers keep their loudness: a unit sine's RMS is 1/√2.
    let loud = mono_rms_at_velocity(1.0);
    assert!(
        (loud - std::f32::consts::FRAC_1_SQRT_2).abs() < 0.05,
        "rms at velocity 1.0 should stay ≈ 0.707, was {loud}"
    );
}

#[test]
fn monophonic_zero_velocity_is_near_silent() {
    let zero = mono_rms_at_velocity(0.0);
    let loud = mono_rms_at_velocity(1.0);
    assert!(
        zero < loud * 0.05,
        "velocity 0.0 should be near-silent (rms = {zero})"
    );
}

#[test]
fn polyphonic_lower_velocity_is_quieter() {
    let loud = poly_rms_at_velocity(1.0);
    let mid = poly_rms_at_velocity(0.5);
    let quiet = poly_rms_at_velocity(0.25);
    assert!(
        loud > 0.1,
        "full-velocity note should produce signal (rms = {loud})"
    );
    assert!(
        mid < loud * 0.95,
        "velocity 0.5 (rms {mid}) should be quieter than 1.0 (rms {loud})"
    );
    assert!(
        quiet < mid * 0.95,
        "velocity 0.25 (rms {quiet}) should be quieter than 0.5 (rms {mid})"
    );
}

#[test]
fn polyphonic_retrigger_applies_new_velocity() {
    // Retriggering the same held note at a lower velocity must use the new
    // velocity, not keep the old one (exercises the in-place retrigger branch).
    let mut source = PolyphonicSource::new(
        steady_sine(),
        4,
        SAMPLE_RATE,
        PolyphonicAllocationStrategy::default(),
    );
    let note = Note::from_midi(69);
    source.start_note(note, 1.0);
    let loud = rms(&source.pull(BLOCK_SIZE));
    source.start_note(note, 0.25);
    let quiet = rms(&source.pull(BLOCK_SIZE));
    assert!(
        quiet < loud * 0.95,
        "retrigger at velocity 0.25 (rms {quiet}) should be quieter than 1.0 (rms {loud})"
    );
}
