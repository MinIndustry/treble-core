use super::Source;
use crate::core::audio::{Block, mono_to_frame, silent_block};
use crate::core::envelope::prelude::{ADSREnvelopeBuilder, BezierSegment, ConstantSegment};
use crate::core::generator::prelude::MultiToneGenerator;

#[derive(Debug, Clone)]
pub struct SimpleSource {
    generator: MultiToneGenerator,
    sample_rate: f32,
    active: bool,
    released: bool,
    // Timing & amplitude
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
    amplitude: f32,
    // Curve scalars in [-1.0, 1.0].
    // 0  → linear (control point at midpoint → standard quadratic bezier degenerates to line)
    // +1 → control lands at `to`   → convex toward the end   (fast start, slow end)
    // -1 → control lands at `from` → convex toward the start (slow start, fast end)
    attack_curve: f32,
    decay_curve: f32,
    release_curve: f32,
    // Horizontal position of each bezier control point in [0, 1] (normalized within segment).
    // 0.5 = midpoint (symmetric), towards 0 = control near segment start, towards 1 = near end.
    attack_cp_t: f32,
    decay_cp_t: f32,
    release_cp_t: f32,
}

/// Maps a [-1, 1] curve scalar to a bézier control-point Y in amplitude space.
///
/// The quadratic bézier used by `BezierSegment` is driven entirely by the Y of its
/// control point; X is parametric (not stored). When `curve = 0`, the result is the
/// midpoint between `from` and `to`, which makes the quadratic bézier degenerate to a
/// straight line — identical to a `LinearSegment`. At `curve = ±1` the control point
/// coincides with one of the endpoints, producing maximum curvature.
fn control_y(from: f32, to: f32, curve: f32) -> f32 {
    from + (curve.clamp(-1.0, 1.0) + 1.0) * 0.5 * (to - from)
}

impl SimpleSource {
    pub fn new(generator: MultiToneGenerator, sample_rate: f32) -> Self {
        let mut src = Self {
            generator,
            sample_rate,
            active: false,
            released: false,
            attack: 0.01,
            decay: 0.1,
            sustain: 0.8,
            release: 0.3,
            amplitude: 0.5,
            attack_curve: 0.0,
            decay_curve: 0.0,
            release_curve: 0.0,
            attack_cp_t: 0.5,
            decay_cp_t: 0.5,
            release_cp_t: 0.5,
        };
        src.apply_envelope();
        src
    }

    pub fn boxed(self) -> Box<dyn Source> {
        Box::new(self)
    }

    /// Rebuilds the global ADSR amplitude envelope from all stored scalars and installs
    /// it on the generator.
    ///
    /// Each timed phase (attack, decay, release) becomes a `BezierSegment` whose
    /// control-point Y is derived from the corresponding curve scalar via `control_y`.
    /// The sustain phase is a `ConstantSegment` of infinite duration — it holds until
    /// note-off regardless of how long that takes.
    fn apply_envelope(&mut self) {
        let a = self.amplitude;
        let s = (self.sustain * a).max(0.0);

        let envelope = ADSREnvelopeBuilder::new()
            .attack(Box::new(BezierSegment::new(
                0.0, a,
                self.attack.max(0.001),
                (self.attack_cp_t, control_y(0.0, a, self.attack_curve)),
            )))
            .decay(Box::new(BezierSegment::new(
                a, s,
                self.decay.max(0.001),
                (self.decay_cp_t, control_y(a, s, self.decay_curve)),
            )))
            .sustain(Box::new(ConstantSegment::new(s, None)))
            .release(Box::new(BezierSegment::new(
                s, 0.0,
                self.release.max(0.001),
                (self.release_cp_t, control_y(s, 0.0, self.release_curve)),
            )))
            .build();

        self.generator.set_global_amplitude_envelope(Box::new(envelope));
    }
}

impl From<MultiToneGenerator> for SimpleSource {
    fn from(generator: MultiToneGenerator) -> Self {
        Self::new(generator, 10.0)
    }
}

impl Source for SimpleSource {
    fn pull(&mut self, block_size: usize) -> Block {
        if !self.active {
            return silent_block(block_size);
        }
        let dt = 1.0 / self.sample_rate;
        let samples = self.generator.tick_block(block_size, dt);
        let max = samples.iter().cloned().fold(0.0_f32, f32::max);
        log::trace!("[SimpleSource] pull(block_size={block_size}) dt={dt:.6} → max_sample={max:.4}");
        if self.released && self.generator.completed() {
            self.active = false;
            self.released = false;
        }
        samples.into_iter().map(mono_to_frame).collect()
    }

    fn start(&mut self) {
        log::info!("[SimpleSource] start() sample_rate={} → active=true", self.sample_rate);
        self.active = true;
        self.released = false;
        self.generator.start();
    }

    fn stop(&mut self) {
        self.generator.stop();
        self.released = true;
    }

    fn kill(&mut self) {
        self.active = false;
        self.released = false;
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn set_parameter(&mut self, name: &str, value: f32) {
        match name {
            "frequency"     => { self.generator.set_base_frequency(value); }
            "amplitude"     => { self.amplitude     = value.max(0.0);          self.apply_envelope(); }
            "attack"        => { self.attack        = value.max(0.001);         self.apply_envelope(); }
            "decay"         => { self.decay         = value.max(0.001);         self.apply_envelope(); }
            "sustain"       => { self.sustain       = value.clamp(0.0, 1.0);   self.apply_envelope(); }
            "release"       => { self.release       = value.max(0.001);         self.apply_envelope(); }
            "attack_curve"  => { self.attack_curve  = value.clamp(-1.0, 1.0);  self.apply_envelope(); }
            "decay_curve"   => { self.decay_curve   = value.clamp(-1.0, 1.0);  self.apply_envelope(); }
            "release_curve" => { self.release_curve = value.clamp(-1.0, 1.0);  self.apply_envelope(); }
            "attack_cp_t"   => { self.attack_cp_t   = value.clamp(0.0, 1.0);   self.apply_envelope(); }
            "decay_cp_t"    => { self.decay_cp_t    = value.clamp(0.0, 1.0);   self.apply_envelope(); }
            "release_cp_t"  => { self.release_cp_t  = value.clamp(0.0, 1.0);   self.apply_envelope(); }
            _ => {}
        }
    }
}

/// Creates a simple source with a given generator and sample rate.
pub fn simple_source(generator: MultiToneGenerator) -> Box<dyn Source> {
    SimpleSource::from(generator).boxed()
}
