use std::fmt;
use std::sync::Arc;

use treble_derive::FilterMetaData;

use crate::core::Block;
use crate::core::graph::{Entry, Filter};

/// Smallest drive the compensation can be computed for.
///
/// The compensation divides by the shaper's own output level, which goes to
/// zero with the drive. At this value the shaper is already indistinguishable
/// from a wire, so nothing is lost by refusing to go lower.
const MIN_DRIVE: f32 = 1e-3;

/// Peak amplitude the gain compensation is calibrated at.
///
/// A soft clipper's gain depends on how hard it is being hit, so no single
/// output trim can hold the level for every input level; the trim has to be
/// calibrated somewhere. This is the engine's own nominal level — the default
/// generator amplitude — so material coming off a default instrument passes
/// through at the level it arrived.
const CALIBRATION_AMPLITUDE: f32 = 0.5;

/// Quadrature points across a quarter period of the calibration sine.
///
/// `tanh` is odd and a sine is quarter-symmetric, so a quarter period carries
/// the whole RMS. Thirty-two midpoints put the result well inside f32.
const CALIBRATION_POINTS: usize = 32;

/// Soft-clip waveshaper. `drive` pushes the signal further into the curve;
/// `mix` blends the result against the dry signal.
///
/// The output is gain-compensated, so turning the drive up adds harmonics at
/// a constant level instead of simply getting louder — without that a soft
/// clipper is a gain knob with extra steps.
#[derive(FilterMetaData, Clone)]
pub struct Saturation {
    #[filter_source]
    source: Arc<Block>,
    /// Pre-gain into the shaper. At 1.0 the curve is very nearly a straight
    /// line; the top of the range is a hard square-up.
    #[filter_parameter(range, 1.0, 32.0, 3.0)]
    drive: f32,
    #[filter_parameter(range, 0.0, 1.0, 1.0)]
    mix: f32,
    /// Output trim that holds the level across the drive range, and the drive
    /// it was computed for. The derived `set_parameter` assigns fields without
    /// recomputing anything, so `transform` is the only place that can notice
    /// a change — the same arrangement the resonant pass filters use for their
    /// coefficients.
    compensation: f32,
    calibrated_for: f32,
}

impl Saturation {
    pub fn new(drive: f32, mix: f32) -> Self {
        let drive = drive.max(MIN_DRIVE);
        Self {
            source: Arc::new(Vec::new()),
            drive,
            mix,
            compensation: Self::compensation(drive),
            calibrated_for: drive,
        }
    }

    /// The output trim that leaves a `CALIBRATION_AMPLITUDE` sine at the RMS it
    /// came in with.
    ///
    /// Measured rather than derived: the RMS of `tanh` of a sine has no
    /// closed form, and the obvious analytic alternative — dividing by
    /// `tanh(drive)` so full scale maps to full scale — leaves quiet material
    /// more than 10 dB louder at high drive, which is exactly the "it is just
    /// a gain knob" complaint this exists to answer. Runs only when `drive`
    /// changes, so at worst once per block under automation.
    fn compensation(drive: f32) -> f32 {
        let mut sum = 0.0f64;
        for point in 0..CALIBRATION_POINTS {
            let phase =
                (point as f32 + 0.5) / CALIBRATION_POINTS as f32 * std::f32::consts::FRAC_PI_2;
            let shaped = (drive * CALIBRATION_AMPLITUDE * phase.sin()).tanh() as f64;
            sum += shaped * shaped;
        }
        let shaped_rms = (sum / CALIBRATION_POINTS as f64).sqrt() as f32;
        CALIBRATION_AMPLITUDE * std::f32::consts::FRAC_1_SQRT_2 / shaped_rms.max(1e-9)
    }
}

impl Default for Saturation {
    fn default() -> Self {
        Self::new(3.0, 1.0)
    }
}

impl Entry for Saturation {
    fn push(&mut self, block: Arc<Block>, _port: usize) {
        self.source = block;
    }
}

impl fmt::Display for Saturation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Saturation - drive: {}, mix: {}",
            self.drive, self.mix
        )
    }
}

impl fmt::Debug for Saturation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Saturation")
            .field("drive", &self.drive)
            .field("mix", &self.mix)
            .finish()
    }
}

impl Filter for Saturation {
    fn transform(&mut self) -> Vec<Block> {
        let drive = self.drive.max(MIN_DRIVE);
        if drive != self.calibrated_for {
            self.compensation = Self::compensation(drive);
            self.calibrated_for = drive;
        }
        let compensation = self.compensation;
        let mix = self.mix.clamp(0.0, 1.0);

        // `tanh` rather than a polynomial or rational approximation: it is
        // strictly monotonic and asymptotic to ±1 over the whole real line, so
        // no input level can fold the waveform back on itself. The cheap
        // approximations are only well behaved on the interval they were fitted
        // to, and a saturator is precisely the filter that gets fed values
        // outside it. Serial, like the rest of the per-sample filters — see
        // GainFilter on why rayon per block is a pessimisation.
        let output: Block = self
            .source
            .iter()
            .map(|frame| {
                std::array::from_fn(|channel| {
                    let shaped = (drive * frame[channel]).tanh() * compensation;
                    frame[channel] * (1.0 - mix) + shaped * mix
                })
            })
            .collect();

        vec![output]
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::CHANNELS;
    use treble_meta::MetaFilter;

    const RATE: f32 = 1000.0;
    /// Ten whole periods inside the test block, so the analysis bins land on
    /// the harmonics exactly and nothing leaks between them.
    const TONE: f32 = 10.0;
    const FRAMES: usize = 1000;

    fn sine(amplitude: f32, frequency: f32) -> Block {
        (0..FRAMES)
            .map(|index| {
                let phase = std::f32::consts::TAU * frequency * index as f32 / RATE;
                [amplitude * phase.sin(); CHANNELS]
            })
            .collect()
    }

    fn run(filter: &mut Saturation, block: Block) -> Block {
        filter.push(Arc::new(block), 0);
        filter.transform().remove(0)
    }

    fn rms(block: &Block, channel: usize) -> f32 {
        let sum: f64 = block
            .iter()
            .map(|frame| (frame[channel] as f64) * (frame[channel] as f64))
            .sum();
        (sum / block.len() as f64).sqrt() as f32
    }

    /// Amplitude of the component at `frequency`, by single-bin DFT.
    fn magnitude(block: &Block, channel: usize, frequency: f32) -> f32 {
        let mut real = 0.0f64;
        let mut imaginary = 0.0f64;
        for (index, frame) in block.iter().enumerate() {
            let angle = std::f64::consts::TAU * frequency as f64 * index as f64 / RATE as f64;
            real += frame[channel] as f64 * angle.cos();
            imaginary -= frame[channel] as f64 * angle.sin();
        }
        (2.0 * real.hypot(imaginary) / block.len() as f64) as f32
    }

    #[test]
    fn drive_adds_harmonics_without_changing_the_level() {
        let input = sine(CALIBRATION_AMPLITUDE, TONE);
        let reference = rms(&input, 0);

        let mut previous_distortion = 0.0f32;
        for drive in [1.0f32, 2.0, 4.0, 8.0, 16.0, 32.0] {
            let mut filter = Saturation::new(drive, 1.0);
            let output = run(&mut filter, input.clone());

            let level = rms(&output, 0);
            let decibels = 20.0 * (level / reference).log10();
            assert!(
                decibels.abs() < 0.2,
                "drive {drive} moved the level by {decibels:.2} dB"
            );

            // The harmonics have to be arriving, or holding the level would be
            // a triumph of doing nothing.
            let fundamental = magnitude(&output, 0, TONE);
            let third = magnitude(&output, 0, TONE * 3.0);
            let distortion = third / fundamental;
            assert!(
                distortion > previous_distortion,
                "drive {drive} produced {distortion:.4} third-harmonic, no more \
                 than the {previous_distortion:.4} of the drive below it"
            );
            previous_distortion = distortion;

            // An odd shaper makes odd harmonics only. An even one here would
            // mean an asymmetry — a DC offset or a folded curve.
            let second = magnitude(&output, 0, TONE * 2.0);
            assert!(
                second < fundamental * 1e-3,
                "drive {drive} produced an even harmonic at {second}"
            );
        }
        assert!(
            previous_distortion > 0.25,
            "the top of the drive range should be close to a square wave's \
             one-third, measured {previous_distortion:.3}"
        );
    }

    /// The check that the compensation is doing the work: the raw shaper on the
    /// same sweep is a gain knob at the bottom of the range.
    #[test]
    fn the_uncompensated_shaper_would_be_a_gain_knob() {
        let input = sine(CALIBRATION_AMPLITUDE, TONE);
        let reference = rms(&input, 0);
        let raw: Block = input
            .iter()
            .map(|frame| std::array::from_fn(|channel| (8.0 * frame[channel]).tanh()))
            .collect();
        let decibels = 20.0 * (rms(&raw, 0) / reference).log10();
        assert!(
            decibels > 6.0,
            "a drive of 8 with no compensation should be well over 6 dB up, \
             measured {decibels:.2} dB"
        );
    }

    #[test]
    fn the_curve_never_turns_back_on_itself() {
        // A shaper that folds back inverts part of the waveform, which sounds
        // like a bug rather than like distortion. Swept well past full scale,
        // because nothing upstream guarantees the input is inside it.
        //
        // Approaching the asymptote an f32 `tanh` runs out of mantissa — by
        // `|drive * x| ~ 9` it returns exactly 1.0 — so the curve legitimately
        // flattens there. Strict increase is therefore only asked for while the
        // curve is still resolving; the requirement everywhere is that it never
        // *falls*.
        const RESOLVED: f32 = 4.0;
        for drive in [1.0f32, 8.0, 32.0] {
            let mut filter = Saturation::new(drive, 1.0);
            let steps = 4001;
            let inputs: Vec<f32> = (0..steps)
                .map(|step| -4.0 + 8.0 * step as f32 / (steps - 1) as f32)
                .collect();
            let sweep: Block = inputs.iter().map(|value| [*value; CHANNELS]).collect();
            let curve = run(&mut filter, sweep);
            for (index, window) in curve.windows(2).enumerate() {
                assert!(
                    window[1][0] >= window[0][0],
                    "drive {drive}: the curve turned back at {} -> {}",
                    window[0][0],
                    window[1][0]
                );
                if (drive * inputs[index + 1]).abs() < RESOLVED {
                    assert!(
                        window[1][0] > window[0][0],
                        "drive {drive}: the curve stalled at input {}",
                        inputs[index + 1]
                    );
                }
            }
            let peak = curve
                .iter()
                .map(|frame| frame[0].abs())
                .fold(0.0f32, f32::max);
            assert!(peak < 2.0, "drive {drive} peaked at {peak}");
        }
    }

    #[test]
    fn a_low_drive_is_nearly_transparent() {
        // The bottom of the range has to be usable as "off", or the filter
        // cannot be left in a chain and turned down.
        let mut filter = Saturation::new(MIN_DRIVE, 1.0);
        let input = sine(CALIBRATION_AMPLITUDE, TONE);
        let expected = input.clone();
        let output = run(&mut filter, input);
        for (frame, (a, b)) in expected.iter().zip(output.iter()).enumerate() {
            assert!(
                (a[0] - b[0]).abs() < 1e-4,
                "frame {frame}: {} became {}",
                a[0],
                b[0]
            );
        }
    }

    #[test]
    fn zero_mix_is_a_bypass() {
        let mut filter = Saturation::new(32.0, 0.0);
        let input = sine(0.9, TONE);
        let expected = input.clone();
        let output = run(&mut filter, input);
        for (frame, (a, b)) in expected.iter().zip(output.iter()).enumerate() {
            assert!((a[0] - b[0]).abs() < 1e-6, "frame {frame} was altered");
        }
    }

    #[test]
    fn a_drive_edit_recalibrates_the_compensation() {
        // Automation moves parameters through the derived setter, which cannot
        // recompute anything; if `transform` did not notice, a swept drive
        // would keep the trim of whatever drive the filter was built with.
        let input = sine(CALIBRATION_AMPLITUDE, TONE);
        let reference = rms(&input, 0);

        let mut filter = Saturation::new(1.0, 1.0);
        let _ = run(&mut filter, input.clone());
        assert!(filter.set_parameter("drive", 24.0));
        let output = run(&mut filter, input);
        let decibels = 20.0 * (rms(&output, 0) / reference).log10();
        assert!(
            decibels.abs() < 0.2,
            "after the edit the level was {decibels:.2} dB off"
        );
    }

    #[test]
    fn the_engine_can_inject_every_parameter_by_name() {
        let mut filter = Saturation::default();
        for name in ["drive", "mix"] {
            assert!(
                filter.supports_parameter(name),
                "'{name}' must be settable from an FxSpec"
            );
        }
        assert!(!filter.set_parameter("nonsense", 1.0));
        // Out-of-range values clamp rather than divide by nothing.
        assert!(filter.set_parameter("drive", -5.0));
        let output = run(&mut filter, sine(0.5, TONE));
        assert!(output.iter().all(|frame| frame[0].is_finite()));
    }
}
