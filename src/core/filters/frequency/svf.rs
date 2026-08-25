//! The 2-pole state-variable filter core shared by the resonant pass filters.

use crate::core::CHANNELS;

/// Lowest resonance the pass filters accept.
///
/// This is critical damping — two coincident real poles, the gentlest a 2-pole
/// gets. Below it the response merely droops without becoming anything a
/// performer would ask for by name.
pub const MIN_RESONANCE: f32 = 0.5;

/// Highest resonance the pass filters accept.
///
/// Holds the damping term `k = 1/Q` at 0.05, which rings hard — roughly +26 dB
/// at the cutoff — but still decays. `k = 0` is deliberately out of reach: an
/// undamped SVF driven at its own cutoff frequency grows without bound, and a
/// performer sweeping into that would take the output with them. Self-
/// oscillation as a *sound* lives just under this ceiling; self-oscillation as
/// an arithmetic accident does not.
pub const MAX_RESONANCE: f32 = 20.0;

/// Butterworth Q: maximally flat, no resonant peak.
pub const DEFAULT_RESONANCE: f32 = std::f32::consts::FRAC_1_SQRT_2;

/// Highest cutoff-to-rate ratio the prewarp is allowed to see.
///
/// `g = tan(PI * ratio)` diverges at 0.5 and turns *negative* past it, which
/// inverts the feedback sign and detonates the filter. The parameter ranges
/// permit a 20 kHz cutoff on an 8 kHz graph, so this clamp is load-bearing
/// rather than decorative; at any ordinary rate the cutoff's own 20 kHz
/// ceiling binds first and the clamp never applies.
const MAX_FREQUENCY_RATIO: f32 = 0.49;

/// Topology-preserving-transform state-variable filter, in the form Andrew
/// Simper (Cytomic) published.
///
/// One structure yields low-pass, band-pass and high-pass from the same pair
/// of integrator states, for three multiplies and six adds per sample per
/// channel. It is used in preference to a direct-form biquad because its
/// states *are* the band-pass and low-pass signals: they stay bounded by the
/// signal itself instead of growing as the poles crowd DC. That is what lets
/// `f32` state hold up at a 20 Hz cutoff, and what lets the cutoff be
/// re-derived every block — which parameter automation now does routinely —
/// without the blow-ups a direct form suffers under modulated coefficients.
#[derive(Clone, Copy, Debug)]
pub struct Svf {
    a1: f32,
    a2: f32,
    a3: f32,
    /// Damping, `1/Q`. Kept because the high-pass output needs it directly.
    k: f32,
    /// Per-channel integrator state, `[ic1eq, ic2eq]`.
    state: [[f32; 2]; CHANNELS],
}

impl Svf {
    pub fn new(cutoff_frequency: f32, resonance: f32, sample_rate: f32) -> Self {
        let mut filter = Self {
            a1: 0.0,
            a2: 0.0,
            a3: 0.0,
            k: 0.0,
            state: [[0.0; 2]; CHANNELS],
        };
        filter.set_coefficients(cutoff_frequency, resonance, sample_rate);
        filter
    }

    /// Recomputes the coefficients. Callers do this when a parameter has
    /// changed, never per sample — the `tan` alone costs more than the filter.
    pub fn set_coefficients(&mut self, cutoff_frequency: f32, resonance: f32, sample_rate: f32) {
        let ratio = (cutoff_frequency / sample_rate.max(1.0)).clamp(1e-6, MAX_FREQUENCY_RATIO);
        let g = (std::f32::consts::PI * ratio).tan();
        self.k = 1.0 / resonance.clamp(MIN_RESONANCE, MAX_RESONANCE);
        self.a1 = 1.0 / (1.0 + g * (g + self.k));
        self.a2 = g * self.a1;
        self.a3 = g * self.a2;
    }

    /// Advances one channel by one sample, returning the band-pass and
    /// low-pass integrator outputs `(v1, v2)`.
    #[inline]
    fn step(&mut self, channel: usize, input: f32) -> (f32, f32) {
        let [ic1eq, ic2eq] = self.state[channel];
        let v3 = input - ic2eq;
        let v1 = self.a1 * ic1eq + self.a2 * v3;
        let v2 = ic2eq + self.a2 * ic1eq + self.a3 * v3;
        self.state[channel] = [2.0 * v1 - ic1eq, 2.0 * v2 - ic2eq];
        (v1, v2)
    }

    #[inline]
    pub fn low_pass(&mut self, channel: usize, input: f32) -> f32 {
        self.step(channel, input).1
    }

    #[inline]
    pub fn band_pass(&mut self, channel: usize, input: f32) -> f32 {
        self.step(channel, input).0
    }

    #[inline]
    pub fn high_pass(&mut self, channel: usize, input: f32) -> f32 {
        let (v1, v2) = self.step(channel, input);
        input - self.k * v1 - v2
    }

    /// Zeroes the integrators if they have gone non-finite.
    ///
    /// Run once per block, not per sample. A NaN reaching the state is
    /// self-sustaining, so one bad input frame would otherwise mute this node
    /// for the rest of the performance; checking per block bounds the damage
    /// to a block and costs four comparisons rather than four per sample.
    pub fn reset_if_unstable(&mut self) {
        if self.state.iter().flatten().any(|value| !value.is_finite()) {
            self.reset();
        }
    }

    /// Clears the integrators, so the next sample starts from silence.
    pub fn reset(&mut self) {
        self.state = [[0.0; 2]; CHANNELS];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Tap = fn(&mut Svf, usize, f32) -> f32;

    /// RMS of one `tap`'s response to a sine at `frequency`, after letting the
    /// transient settle.
    fn response(mut filter: Svf, frequency: f32, sample_rate: f32, tap: Tap) -> f32 {
        let frames = (sample_rate * 0.5) as usize;
        let mut sum = 0.0f64;
        let mut counted = 0usize;
        for index in 0..frames {
            let phase = std::f32::consts::TAU * frequency * index as f32 / sample_rate;
            let value = tap(&mut filter, 0, phase.sin());
            // Skip the first tenth: the integrators are still charging.
            if index > frames / 10 {
                sum += (value as f64) * (value as f64);
                counted += 1;
            }
        }
        (sum / counted as f64).sqrt() as f32
    }

    #[test]
    fn the_low_pass_rolls_off_at_twelve_decibels_per_octave() {
        let rate = 48_000.0;
        let cutoff = 500.0;
        let build = || Svf::new(cutoff, DEFAULT_RESONANCE, rate);
        // Two octaves above the cutoff should be ~24 dB down on one octave
        // above it. A one-pole would manage half that.
        let one_octave = response(build(), cutoff * 2.0, rate, Svf::low_pass);
        let three_octaves = response(build(), cutoff * 8.0, rate, Svf::low_pass);
        let decibels = 20.0 * (one_octave / three_octaves).log10();
        assert!(
            (decibels - 24.0).abs() < 2.0,
            "expected ~24 dB over two octaves, measured {decibels:.1} dB"
        );
    }

    #[test]
    fn resonance_peaks_at_the_cutoff() {
        let rate = 48_000.0;
        let cutoff = 1_000.0;
        let flat = response(
            Svf::new(cutoff, DEFAULT_RESONANCE, rate),
            cutoff,
            rate,
            Svf::low_pass,
        );
        let peaked = response(Svf::new(cutoff, 8.0, rate), cutoff, rate, Svf::low_pass);
        let gain = 20.0 * (peaked / flat).log10();
        assert!(
            gain > 15.0,
            "Q=8 should lift the cutoff by well over 15 dB, measured {gain:.1} dB"
        );
    }

    /// The band-pass tap comes free from the same integrators. Nothing in the
    /// registry exposes it yet, so this is the only thing holding it honest.
    #[test]
    fn the_band_pass_tap_rejects_both_sides_of_the_centre() {
        let rate = 48_000.0;
        let centre = 1_000.0;
        let build = || Svf::new(centre, 4.0, rate);
        let at_centre = response(build(), centre, rate, Svf::band_pass);
        let two_below = response(build(), centre / 4.0, rate, Svf::band_pass);
        let two_above = response(build(), centre * 4.0, rate, Svf::band_pass);
        assert!(
            at_centre > two_below * 8.0 && at_centre > two_above * 8.0,
            "centre {at_centre}, two octaves down {two_below}, two octaves up {two_above}"
        );
    }

    #[test]
    fn the_high_pass_tap_is_the_low_pass_response_mirrored() {
        let rate = 48_000.0;
        let cutoff = 1_000.0;
        let build = || Svf::new(cutoff, DEFAULT_RESONANCE, rate);
        // Butterworth is -3 dB at the cutoff on both taps, so the two agree
        // there and diverge on either side.
        let low_at_cutoff = response(build(), cutoff, rate, Svf::low_pass);
        let high_at_cutoff = response(build(), cutoff, rate, Svf::high_pass);
        assert!(
            (low_at_cutoff - high_at_cutoff).abs() < 0.05 * low_at_cutoff,
            "low {low_at_cutoff} vs high {high_at_cutoff} at the cutoff"
        );
        let high_well_below = response(build(), cutoff / 8.0, rate, Svf::high_pass);
        assert!(
            high_well_below < low_at_cutoff * 0.05,
            "three octaves below the cutoff the high-pass should be gone, got {high_well_below}"
        );
    }

    #[test]
    fn a_cutoff_above_nyquist_stays_stable() {
        // A spec may legitimately hold a 20 kHz cutoff while the graph runs at
        // 8 kHz; the prewarp clamp is the only thing standing between that and
        // an inverted feedback sign.
        let mut filter = Svf::new(20_000.0, MAX_RESONANCE, 8_000.0);
        for index in 0..8_000 {
            let value = filter.low_pass(0, if index % 2 == 0 { 1.0 } else { -1.0 });
            assert!(
                value.is_finite() && value.abs() < 100.0,
                "diverged: {value}"
            );
        }
    }

    #[test]
    fn a_nan_does_not_latch_into_the_state() {
        let mut filter = Svf::new(1_000.0, DEFAULT_RESONANCE, 48_000.0);
        let _ = filter.low_pass(0, f32::NAN);
        filter.reset_if_unstable();
        assert!(filter.low_pass(0, 1.0).is_finite());
    }
}
