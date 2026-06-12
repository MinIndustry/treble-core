//! Sample-accurate note scheduling tests (BUG-004).
//!
//! `render_block` must split a block at scheduled event frames so a note
//! starts on exactly its frame — not at the next block boundary.

use treble::Note;
use treble::audio::{EventScheduler, InstrumentAudioMessage, render_block};
use treble::core::envelope::prelude::ConstantSegment;
use treble::core::generator::prelude::{
    FrequencyRelation, Waveform,
    builder::{MultiToneGeneratorBuilder, ToneGeneratorBuilder},
};
use treble::core::graph::{MonophonicAllocationStrategy, MonophonicSource, SimpleSink, System};

const BLOCK_SIZE: usize = 512; // System default

fn note_start(at_velocity: f32) -> InstrumentAudioMessage {
    InstrumentAudioMessage::NoteStart {
        source_index: 0,
        note: Note::from_midi(69),
        velocity: at_velocity,
    }
}

/// System with one steady-sine mono source wired straight to a SimpleSink.
fn sine_system() -> System {
    let generator = MultiToneGeneratorBuilder::new()
        .add_generator(
            ToneGeneratorBuilder::new()
                .waveform(Waveform::Sine)
                .frequency_relation(FrequencyRelation::Identity)
                .amplitude_envelope(Box::new(ConstantSegment::new(1.0, None)))
                .build(),
        )
        .frequency(440.0)
        .build();
    let mut system = System::new();
    let src = system.add_source(Box::new(MonophonicSource::new(
        generator,
        44100.0,
        MonophonicAllocationStrategy::Replace,
    )));
    let sink = system.add_sink(Box::new(SimpleSink::new()));
    system.connect_source_to_sink(src, sink);
    system.compute().expect("graph compute failed");
    system
}

fn rms(samples: &[f32]) -> f32 {
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
}

#[test]
fn scheduler_pops_in_frame_order() {
    let mut sched = EventScheduler::new();
    sched.schedule(300, note_start(0.3));
    sched.schedule(100, note_start(0.1));
    sched.schedule(200, note_start(0.2));

    let mut velocities = Vec::new();
    while let Some(InstrumentAudioMessage::NoteStart { velocity, .. }) = sched.pop_due(u64::MAX) {
        velocities.push(velocity);
    }
    assert_eq!(velocities, vec![0.1, 0.2, 0.3], "events must pop by frame");
}

#[test]
fn scheduler_same_frame_keeps_submission_order() {
    let mut sched = EventScheduler::new();
    sched.schedule(100, note_start(0.1));
    sched.schedule(100, note_start(0.2));
    sched.schedule(100, note_start(0.3));

    let mut velocities = Vec::new();
    while let Some(InstrumentAudioMessage::NoteStart { velocity, .. }) = sched.pop_due(100) {
        velocities.push(velocity);
    }
    assert_eq!(
        velocities,
        vec![0.1, 0.2, 0.3],
        "same-frame events are FIFO"
    );
}

#[test]
fn scheduler_pop_due_respects_the_deadline() {
    let mut sched = EventScheduler::new();
    sched.schedule(100, note_start(1.0));
    assert!(sched.pop_due(99).is_none(), "frame 100 is not due at 99");
    assert!(sched.pop_due(100).is_some(), "frame 100 is due at 100");
    assert!(sched.is_empty());
}

#[test]
fn note_starts_on_exactly_its_frame() {
    let mut system = sine_system();
    let mut sched = EventScheduler::new();
    sched.schedule(100, note_start(1.0));

    let mut out = Vec::new();
    let next = render_block(&mut system, &mut sched, 0, &mut out);

    assert_eq!(next, BLOCK_SIZE as u64, "clock advances one block");
    assert_eq!(out.len(), BLOCK_SIZE * 2, "stereo-interleaved full block");

    // Frames 0..100 (samples 0..200): the note has not started — exact silence.
    assert!(
        out[..200].iter().all(|&s| s == 0.0),
        "audio before the scheduled frame must be silent"
    );
    // Frames 100..512: the note is playing.
    assert!(
        rms(&out[200..]) > 0.3,
        "audio from the scheduled frame on must be loud (rms = {})",
        rms(&out[200..])
    );
}

#[test]
fn event_at_block_end_waits_for_the_next_block() {
    let mut system = sine_system();
    let mut sched = EventScheduler::new();
    sched.schedule(BLOCK_SIZE as u64, note_start(1.0)); // due at frame 512

    let mut out = Vec::new();
    let next = render_block(&mut system, &mut sched, 0, &mut out);
    assert!(
        out.iter().all(|&s| s == 0.0),
        "block [0, 512) must not contain the frame-512 event"
    );
    assert_eq!(sched.len(), 1, "event still pending");

    out.clear();
    render_block(&mut system, &mut sched, next, &mut out);
    assert!(sched.is_empty(), "event consumed at the start of block two");
    assert!(rms(&out) > 0.3, "note plays from frame 512");
}

#[test]
fn late_event_applies_at_block_start() {
    let mut system = sine_system();
    let mut sched = EventScheduler::new();
    sched.schedule(50, note_start(1.0)); // already in the past

    let mut out = Vec::new();
    render_block(&mut system, &mut sched, 100, &mut out);
    assert!(
        rms(&out[..200]) > 0.3,
        "late event must apply before the first sample of the block"
    );
}
