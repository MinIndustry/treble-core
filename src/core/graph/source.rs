use crate::core::audio::Block;
use crate::core::utils::Note;
use dyn_clone::DynClone;

/// The source trait defines node that can be used as source in the audio graph.
pub trait Source: std::fmt::Debug + DynClone + Send {
    /// Pull exactly one Block (block_size frames) of stereo audio.
    /// The block_size is known to the source via the System that owns it.
    fn pull(&mut self, block_size: usize) -> Block;

    /// Soft start, envelope attack begins
    fn start(&mut self) {}
    /// Soft stop, let the release phase of the envelope finish
    fn stop(&mut self) {}
    /// Hard stop, don't care about the release phase
    fn kill(&mut self) {}

    /// Note-aware start (defaults to start()).
    fn start_note(&mut self, _note: Note, _velocity: f32) {
        self.start();
    }
    /// Note-aware stop (defaults to stop()).
    fn stop_note(&mut self, _note: Note) {
        self.stop();
    }

    /// True while the source is producing non-silent output (or during release).
    fn is_active(&self) -> bool {
        false
    }
}
dyn_clone::clone_trait_object!(Source);
