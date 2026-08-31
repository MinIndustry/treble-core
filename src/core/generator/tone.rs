use core::f32;
use rand::rngs::SmallRng;
use rand::{self, Rng, SeedableRng};
use serde::{Deserialize, Serialize};

/// Seed for a generator's noise stream, from its identity and which voice it is.
///
/// Each generator needs its own stream — `thread_rng` per sample was a
/// thread-local lookup in the hottest loop (BN-003), and voices sharing a seed
/// emit identical noise, which sums to a +6 dB doubling instead of a thicker
/// texture. It used to come from a global counter, which decorrelated the
/// voices but also meant two renders of one buffer in the same process drew
/// different noise. The voice index does the same job and reproduces.
fn noise_seed(frequency: f32, waveform: &Waveform, voice: u64) -> u64 {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    (
        "noise",
        frequency.to_bits(),
        std::mem::discriminant(waveform),
        voice,
    )
        .hash(&mut hasher);
    hasher.finish()
}

/// The stream a generator gets when nothing has said which voice it is.
fn default_noise_rng() -> SmallRng {
    SmallRng::seed_from_u64(0x9E37_79B9_7F4A_7C15)
}

/// The phase a fresh oscillator starts at, in radians.
///
/// Oscillators want spreading out — a stack that all start at zero sums
/// coherently on the attack and reads as one loud voice rather than several —
/// but this used to be `rand::random()`, drawn from thread entropy, which made
/// every render of the same buffer a different file. The language promises the
/// opposite: a buffer replays identically, and a renderer has to be able to
/// produce the same WAV twice.
///
/// Derived from the tone's own identity instead: different partials of an
/// instrument land in different places because their frequencies differ, and
/// the same tone lands in the same place every run. Two oscillators genuinely
/// at the same frequency now start together, which is what unison should do.
///
/// The old line also had the units wrong — `rand::random::<f32>()` returns
/// `[0, 1)` and `.rem(360.0)` left it there, so a value meant as degrees was
/// used as radians and spread the phase over one sixth of the cycle.
fn spread_phase(frequency: f32, waveform: &Waveform) -> f32 {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    // `to_bits` so the hash is over the exact value, not a lossy rendering.
    frequency.to_bits().hash(&mut hasher);
    std::mem::discriminant(waveform).hash(&mut hasher);
    let unit = (hasher.finish() >> 11) as f64 / (1u64 << 53) as f64;
    (unit as f32) * std::f32::consts::TAU
}

use crate::core::{envelope::Envelope, generator::prelude::*};

use super::composite_builder;

/// Quadratic polyBLEP correction for value discontinuities (sawtooth, square).
/// `t`: normalized phase [0, 1); `dt`: phase increment per sample.
/// Subtract from sawtooth; add/subtract at each square-wave edge.
fn poly_blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let t = t / dt;
        2.0 * t - t * t - 1.0
    } else if t > 1.0 - dt {
        let t = (t - 1.0) / dt;
        t * t + 2.0 * t + 1.0
    } else {
        0.0
    }
}

/// Cubic polyBLAMP correction for slope discontinuities (triangle).
/// Integral of poly_blep; smooths the ±4-slope kinks at t=0 and t=0.5.
fn poly_blamp(t: f32, dt: f32) -> f32 {
    if t < dt {
        let t = t / dt - 1.0;
        -dt / 3.0 * t * t * t
    } else if t > 1.0 - dt {
        let t = (t - 1.0) / dt + 1.0;
        dt / 3.0 * t * t * t
    } else {
        0.0
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SingleToneGenerator {
    waveform: Waveform,
    frequency_relation: Option<FrequencyRelation>,
    pitch_envelope: Option<Box<dyn Envelope>>,
    amplitude_envelope: Box<dyn Envelope>,
    phase: f32,
    note_off: Option<f32>, // Time when the note turned off (stop was called)
    time: f32,
    current_frequency: f32,
    pink_b: [f32; 7], // IIR filter state for pink noise (Paul Kellet algorithm)
    // A deserialised generator has no history to restore; it gets voice 0's
    // stream and whoever pools it says otherwise.
    #[serde(skip, default = "default_noise_rng")]
    rng: SmallRng,
    /// Which voice of a polyphonic pool this is — the noise stream's index.
    voice: u64,
}

/// Hand-written so a cloned voice reseeds its noise stream — see
/// [`NOISE_SEED`]. Everything else copies verbatim.
impl Clone for SingleToneGenerator {
    fn clone(&self) -> Self {
        Self {
            waveform: self.waveform.clone(),
            frequency_relation: self.frequency_relation.clone(),
            pitch_envelope: self.pitch_envelope.clone(),
            amplitude_envelope: self.amplitude_envelope.clone(),
            phase: self.phase,
            note_off: self.note_off,
            time: self.time,
            current_frequency: self.current_frequency,
            pink_b: self.pink_b,
            voice: self.voice,
            // Copied, not redrawn: a clone of a generator is the same
            // generator until whoever made it says which voice it is.
            rng: self.rng.clone(),
        }
    }
}

impl SingleToneGenerator {
    pub fn new(
        waveform: Waveform,
        frequency_relation: Option<FrequencyRelation>,
        pitch_envelope: Option<Box<dyn Envelope>>,
        amplitude_envelope: Box<dyn Envelope>,
        frequency: f32,
    ) -> Self {
        let seed = noise_seed(frequency, &waveform, 0);
        Self {
            phase: spread_phase(frequency, &waveform),
            waveform,
            frequency_relation,
            pitch_envelope,
            amplitude_envelope,
            time: 0.0,
            note_off: None,
            current_frequency: frequency,
            pink_b: [0.0; 7],
            voice: 0,
            rng: SmallRng::seed_from_u64(seed),
        }
    }

    /// Point this generator at voice `voice`'s noise stream.
    ///
    /// Called when a polyphonic pool grows, so the new voice's noise is
    /// uncorrelated with its siblings' without any of them depending on how
    /// many generators the process happened to build before them.
    pub fn set_voice(&mut self, voice: u64) {
        self.voice = voice;
        self.rng =
            SmallRng::seed_from_u64(noise_seed(self.current_frequency, &self.waveform, voice));
    }

    pub fn start(&mut self) {
        self.time = 0.0;
        self.note_off = None;
        self.pink_b = [0.0; 7];
        // Note: We intentionally do NOT reset phase here to avoid phase discontinuities.
        // Each oscillator maintains its phase across note boundaries, which prevents clicks
        // and allows for smooth retriggering. For most musical contexts, this is desirable.
        // For a phase-reset behavior, we can consider adding a separate reset() method.
    }

    pub fn stop(&mut self) {
        self.note_off = Some(self.time);
    }

    pub fn completed(&self) -> bool {
        self.note_off
            .map(|note_off| self.amplitude_envelope.completed(self.time, note_off))
            == Some(true)
    }

    pub fn tick(&mut self, time_elapsed: f32) -> f32 {
        const TAU: f32 = 2.0 * f32::consts::PI;

        // Map true time elapsed for pitch bend
        let actual_elapsed = if let Some(envelope) = &self.pitch_envelope {
            time_elapsed * envelope.at(self.time, self.note_off.unwrap_or(0.0))
        } else {
            time_elapsed
        };
        self.time += time_elapsed;

        // 2 * pi * [[ (t - t0) / T ]]
        if self.waveform.has_frequency() {
            self.phase = (self.phase + TAU * actual_elapsed * self.current_frequency) % TAU;
        }

        // Normalized phase [0, 1) and per-sample phase increment — used by polyBLEP/BLAMP.
        let t = self.phase / TAU;
        let dt = actual_elapsed * self.current_frequency;

        let tone_value = match self.waveform {
            Waveform::Blank | Waveform::Err(_) => 1.0, // Returns 1.0 that will be mapped to the amplitude envelope
            Waveform::PinkNoise => {
                let white = self.rng.gen_range(-1.0_f32..1.0);
                self.pink_b[0] = 0.99886 * self.pink_b[0] + white * 0.0555179;
                self.pink_b[1] = 0.99332 * self.pink_b[1] + white * 0.0750759;
                self.pink_b[2] = 0.96900 * self.pink_b[2] + white * 0.153852;
                self.pink_b[3] = 0.86650 * self.pink_b[3] + white * 0.3104856;
                self.pink_b[4] = 0.55000 * self.pink_b[4] + white * 0.5329522;
                self.pink_b[5] = -0.7616 * self.pink_b[5] - white * 0.0168980;
                self.pink_b[6] = white * 0.115926;
                (self.pink_b.iter().sum::<f32>() + white * 0.5362) * 0.11
            }
            Waveform::Sawtooth => {
                let naive = 2.0 * t - 1.0;
                naive - poly_blep(t, dt)
            }
            Waveform::Sine => f32::sin(self.phase),
            Waveform::Square => {
                let naive = if t < 0.5 { 1.0_f32 } else { -1.0_f32 };
                naive + poly_blep(t, dt) - poly_blep((t + 0.5).rem_euclid(1.0), dt)
            }
            Waveform::Triangle => {
                let naive = if t < 0.5 {
                    4.0 * t - 1.0
                } else {
                    3.0 - 4.0 * t
                };
                naive + 4.0 * (poly_blamp(t, dt) - poly_blamp((t + 0.5).rem_euclid(1.0), dt))
            }
            // Naive (non-band-limited) variants — correct for LFO duty where aliasing is inaudible.
            Waveform::SawRaw => (self.phase * f32::consts::FRAC_1_PI) - 1.0,
            Waveform::SquareRaw => {
                if self.phase > f32::consts::PI {
                    1.0
                } else {
                    -1.0
                }
            }
            Waveform::TriangleRaw => {
                1.0 - 2.0 * ((self.phase * f32::consts::FRAC_1_PI) - 1.0).abs()
            }
            Waveform::WhiteNoise => self.rng.gen_range(-1.0..1.0),
        };

        tone_value
            * self
                .amplitude_envelope
                .at(self.time, self.note_off.unwrap_or(0.0))
    }

    pub fn set_frequency(&mut self, frequency: f32) {
        self.current_frequency = frequency;
    }

    pub fn has_frequency_relation(&self) -> bool {
        self.frequency_relation.is_some()
    }

    pub fn get_waveform(&self) -> &Waveform {
        &self.waveform
    }

    pub fn update_frequency(&mut self, base_frequency: f32) {
        if let Some(relation) = &self.frequency_relation {
            self.current_frequency = relation.compute(base_frequency);
        }
    }
}

impl From<SingleToneGenerator> for MultiToneGenerator {
    fn from(val: SingleToneGenerator) -> Self {
        composite_builder::MultiToneGeneratorBuilder::new()
            .add_generator(val)
            .build()
    }
}

#[cfg(test)]
mod determinism_tests {
    use super::*;

    fn held() -> Box<dyn Envelope> {
        // A flat envelope, so the test sees the oscillator and nothing else.
        Box::new(crate::core::envelope::prelude::ConstantSegment::new(
            1.0, None,
        ))
    }

    fn first_samples(frequency: f32, waveform: Waveform, count: usize) -> Vec<f32> {
        let mut generator = SingleToneGenerator::new(waveform, None, None, held(), frequency);
        generator.start();
        (0..count).map(|_| generator.tick(1.0 / 44_100.0)).collect()
    }

    /// Two generators built from the same description produce the same signal.
    ///
    /// They did not: the start phase came from `rand::random()`, so every
    /// render of the same buffer was a different file. The language promises a
    /// buffer replays identically, and a headless renderer has to be able to
    /// produce the same WAV twice.
    #[test]
    fn the_same_tone_starts_at_the_same_phase() {
        let a = first_samples(440.0, Waveform::Sine, 512);
        let b = first_samples(440.0, Waveform::Sine, 512);
        assert_eq!(a, b, "two identical tones diverged");
    }

    /// Different partials still land in different places, which is why the
    /// phase is spread at all — a stack starting together sums coherently on
    /// the attack and reads as one loud voice.
    #[test]
    fn different_tones_are_still_spread_apart() {
        let a = spread_phase(440.0, &Waveform::Sine);
        let b = spread_phase(880.0, &Waveform::Sine);
        let c = spread_phase(440.0, &Waveform::Sawtooth);
        assert!((a - b).abs() > 1e-6, "440 and 880 share a phase");
        assert!((a - c).abs() > 1e-6, "sine and saw at 440 share a phase");
    }

    /// The phase is in radians, and the whole cycle is available.
    ///
    /// The old line read `rand::random::<f32>().rem(360.0)`: a value meant as
    /// degrees, left in `[0, 1)` by a modulo that could never fire, and then
    /// used as radians — one sixth of the cycle instead of all of it.
    #[test]
    fn the_phase_covers_the_whole_cycle_in_radians() {
        let mut lowest = f32::MAX;
        let mut highest = f32::MIN;
        for step in 0..2000 {
            let phase = spread_phase(40.0 + step as f32 * 3.7, &Waveform::Sine);
            assert!(
                (0.0..std::f32::consts::TAU).contains(&phase),
                "phase {phase} outside [0, TAU)"
            );
            lowest = lowest.min(phase);
            highest = highest.max(phase);
        }
        assert!(lowest < 0.2, "never starts near zero (lowest {lowest})");
        assert!(
            highest > std::f32::consts::TAU - 0.2,
            "never reaches the end of the cycle (highest {highest})"
        );
    }
}
