use std::collections::HashMap;

use crate::core::{Block, utils::Note};

use super::Source;
use crate::instruments::spec::NoteLifecycle;

#[derive(Debug, Clone, Copy)]
enum NoteState {
    Held(usize),
    Releasing(usize),
}

/// Applies the declarative note lifecycle around any mono or poly source.
#[derive(Debug, Clone)]
pub struct NoteLifecycleSource {
    source: Box<dyn Source>,
    lifecycle: NoteLifecycle,
    release_after_frames: usize,
    release_frames: usize,
    notes: HashMap<Note, NoteState>,
}

impl NoteLifecycleSource {
    pub fn new(
        source: Box<dyn Source>,
        lifecycle: NoteLifecycle,
        sample_rate: f32,
        release_after: f32,
        release_duration: f32,
    ) -> Self {
        Self {
            source,
            lifecycle,
            release_after_frames: (release_after.max(0.001) * sample_rate) as usize,
            release_frames: (release_duration.max(0.0) * sample_rate) as usize,
            notes: HashMap::new(),
        }
    }

    fn advance_notes(&mut self, frames: usize) {
        let mut release = Vec::new();
        let mut kill = Vec::new();
        for (&note, state) in &mut self.notes {
            match state {
                NoteState::Held(age) => {
                    *age += frames;
                    if self.lifecycle == NoteLifecycle::OneShot && *age >= self.release_after_frames
                    {
                        release.push(note);
                    }
                }
                NoteState::Releasing(age) => {
                    *age += frames;
                    if *age >= self.release_frames {
                        kill.push(note);
                    }
                }
            }
        }
        for note in release {
            self.source.stop_note(note);
            self.notes.insert(note, NoteState::Releasing(0));
            if self.release_frames == 0 {
                kill.push(note);
            }
        }
        for note in kill {
            self.source.kill_note(note);
            self.notes.remove(&note);
        }
    }
}

impl Source for NoteLifecycleSource {
    fn pull(&mut self, block_size: usize) -> Block {
        // Render the current release interval before advancing its lifetime.
        // Advancing first could kill a voice one entire render segment early.
        let block = self.source.pull(block_size);
        self.advance_notes(block_size);
        block
    }

    fn start(&mut self) {
        self.source.start();
    }
    fn stop(&mut self) {
        match self.lifecycle {
            NoteLifecycle::OneShot => {}
            NoteLifecycle::Gated => self.source.stop(),
            NoteLifecycle::Cutoff => self.source.kill(),
        }
    }
    fn kill(&mut self) {
        self.notes.clear();
        self.source.kill();
    }
    fn start_note(&mut self, note: Note, velocity: f32) {
        self.source.start_note(note, velocity);
        self.notes.insert(note, NoteState::Held(0));
    }
    fn stop_note(&mut self, note: Note) {
        match self.lifecycle {
            NoteLifecycle::OneShot => {}
            NoteLifecycle::Gated => {
                self.source.stop_note(note);
                self.notes.insert(note, NoteState::Releasing(0));
            }
            NoteLifecycle::Cutoff => {
                self.source.kill_note(note);
                self.notes.remove(&note);
            }
        }
    }
    fn kill_note(&mut self, note: Note) {
        self.notes.remove(&note);
        self.source.kill_note(note);
    }
    fn is_active(&self) -> bool {
        self.source.is_active()
    }
    fn supports_parameter(&self, name: &str) -> bool {
        self.source.supports_parameter(name)
    }
    fn set_parameter(&mut self, name: &str, value: f32) -> bool {
        self.source.set_parameter(name, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::audio::silent_block;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[derive(Debug, Clone)]
    struct Probe {
        stops: Arc<AtomicUsize>,
        kills: Arc<AtomicUsize>,
    }
    impl Source for Probe {
        fn stop_note(&mut self, _: Note) {
            self.stops.fetch_add(1, Ordering::Relaxed);
        }
        fn kill_note(&mut self, _: Note) {
            self.kills.fetch_add(1, Ordering::Relaxed);
        }
        fn pull(&mut self, _block_size: usize) -> Block {
            silent_block(_block_size)
        }
    }

    fn wrapped(mode: NoteLifecycle) -> (NoteLifecycleSource, Arc<AtomicUsize>, Arc<AtomicUsize>) {
        let stops = Arc::new(AtomicUsize::new(0));
        let kills = Arc::new(AtomicUsize::new(0));
        (
            NoteLifecycleSource::new(
                Box::new(Probe {
                    stops: stops.clone(),
                    kills: kills.clone(),
                }),
                mode,
                1000.0,
                0.01,
                0.005,
            ),
            stops,
            kills,
        )
    }

    #[test]
    fn one_shot_ignores_external_release_then_finishes() {
        let (mut source, stops, kills) = wrapped(NoteLifecycle::OneShot);
        let note = Note::from_midi(60);
        source.start_note(note, 1.0);
        source.stop_note(note);
        assert_eq!(stops.load(Ordering::Relaxed), 0);
        source.pull(10);
        assert_eq!(stops.load(Ordering::Relaxed), 1);
        source.pull(5);
        assert_eq!(kills.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn gated_releases_and_cutoff_kills() {
        let note = Note::from_midi(60);
        let (mut gated, stops, _) = wrapped(NoteLifecycle::Gated);
        gated.start_note(note, 1.0);
        gated.stop_note(note);
        assert_eq!(stops.load(Ordering::Relaxed), 1);
        let (mut cutoff, _, kills) = wrapped(NoteLifecycle::Cutoff);
        cutoff.start_note(note, 1.0);
        cutoff.stop_note(note);
        assert_eq!(kills.load(Ordering::Relaxed), 1);
    }
}
