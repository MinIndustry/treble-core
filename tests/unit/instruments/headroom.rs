//! Gain staging and spectral balance of the built-in instruments.
//!
//! Two properties are guarded here, because both were wrong at once and each
//! sounded like the other:
//!
//! - **Headroom.** A single note at full velocity used to reach or pass full
//!   scale — a lone kick peaked at +0.6 dBFS — so any mix at all ran into the
//!   master ceiling and stayed there. An instrument has to leave room for the
//!   rest of the arrangement, which is what [`PEAK_WINDOW`] states.
//! - **Top-end shape.** Percussion is built from white noise, which is flat to
//!   Nyquist unless something shapes it. Unshaped, it reads as hiss rather than
//!   as a drum, and it is the part of the spectrum a resampler folds back.
//!   [`hf_share`] measures how much energy sits above 14 kHz.
//!
//! Run with `--nocapture` to print the table rather than only assert on it.

use treble::Note;
use treble::core::Block;
use treble::instruments::prelude::{InstrumentRegistry, compile_spec};

const SAMPLE_RATE: f32 = 44100.0;

/// Where a single note at velocity 1.0 must peak.
///
/// Centred a little under −12 dBFS: low enough that six or eight of these sum
/// without leaning on the ceiling, high enough that one instrument alone is not
/// inaudibly quiet. The band is wide because these are musical judgements, not
/// calibration — it exists to catch an instrument drifting to full scale.
const PEAK_WINDOW: (f32, f32) = (0.10, 0.40);

/// Instruments whose voice is deliberately noise, and the most energy each may
/// put above 14 kHz. A hi-hat lives up there; a kick has no business doing so.
const HF_CEILING: [(&str, f32); 6] = [
    ("kick", 0.02),
    ("snare", 0.10),
    ("hihat", 0.22),
    ("clap", 0.14),
    ("rim", 0.14),
    ("tom", 0.05),
];

const BUILT_INS: [&str; 15] = [
    "kick", "snare", "hihat", "clap", "rim", "tom", "sine", "saw", "square", "triangle", "piano",
    "bass", "pad", "pluck", "bell",
];

/// Render one note of `name` at full velocity and return the left channel.
fn render_one_note(name: &str, seconds: f32) -> Vec<f32> {
    let registry = InstrumentRegistry::built_in();
    let spec = registry.get(name).unwrap_or_else(|| panic!("no '{name}'"));
    let mut system = compile_spec(spec, SAMPLE_RATE).expect("compile");
    system.compute().expect("compute");
    system.start_note(0, Note::new(treble::NOTES::A, 3), 1.0);

    let blocks = (seconds * SAMPLE_RATE / system.block_size() as f32).ceil() as usize;
    (0..blocks)
        .flat_map(|_| {
            system.run();
            system
                .get_sink(0)
                .map(|sink| sink.consume())
                .unwrap_or_else(|_| Block::new())
        })
        .map(|frame| frame[0])
        .collect()
}

fn peak(samples: &[f32]) -> f32 {
    samples.iter().fold(0.0f32, |m, s| m.max(s.abs()))
}

/// The share of spectral energy above `hz`, via a plain Goertzel-free DFT over
/// one window at the loudest point — enough resolution to tell a shaped drum
/// from raw noise without pulling in an FFT dependency.
fn hf_share(samples: &[f32], hz: f32) -> f32 {
    const N: usize = 2048;
    let onset = samples
        .iter()
        .position(|s| s.abs() > peak(samples) * 0.5)
        .unwrap_or(0);
    let window: Vec<f32> = samples
        .iter()
        .skip(onset)
        .take(N)
        .copied()
        .chain(std::iter::repeat(0.0))
        .take(N)
        .collect();
    // Hann, so the transient's edges do not smear energy across the spectrum.
    let windowed: Vec<f32> = window
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let w = 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / (N as f32 - 1.0)).cos();
            s * w
        })
        .collect();

    let mut total = 0.0f64;
    let mut high = 0.0f64;
    // Only every 4th bin: the ratio is what matters and this keeps the test fast.
    for k in (1..N / 2).step_by(4) {
        let freq = k as f32 * SAMPLE_RATE / N as f32;
        let (mut re, mut im) = (0.0f64, 0.0f64);
        let step = 2.0 * std::f64::consts::PI * k as f64 / N as f64;
        for (i, s) in windowed.iter().enumerate() {
            let angle = step * i as f64;
            re += *s as f64 * angle.cos();
            im -= *s as f64 * angle.sin();
        }
        let magnitude = (re * re + im * im).sqrt();
        total += magnitude;
        if freq > hz {
            high += magnitude;
        }
    }
    if total <= 0.0 {
        return 0.0;
    }
    (high / total) as f32
}

#[test]
fn every_built_in_leaves_headroom_for_a_mix() {
    let mut failures = Vec::new();
    println!(
        "\n{:<10}{:>9}{:>10}{:>11}",
        "instrument", "peak", "dBFS", ">14kHz"
    );
    for name in BUILT_INS {
        let samples = render_one_note(name, 1.5);
        let p = peak(&samples);
        let share = hf_share(&samples, 14_000.0);
        println!(
            "{name:<10}{p:9.3}{:10.1}{:10.1}%",
            20.0 * p.max(1e-9).log10(),
            100.0 * share
        );
        if p < PEAK_WINDOW.0 || p > PEAK_WINDOW.1 {
            failures.push(format!(
                "'{name}' peaks at {p:.3} ({:.1} dBFS), outside {:?}",
                20.0 * p.max(1e-9).log10(),
                PEAK_WINDOW
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "headroom:\n  {}",
        failures.join("\n  ")
    );
}

#[test]
fn percussion_noise_is_shaped_rather_than_flat_to_nyquist() {
    let mut failures = Vec::new();
    for (name, ceiling) in HF_CEILING {
        let samples = render_one_note(name, 1.0);
        let share = hf_share(&samples, 14_000.0);
        if share > ceiling {
            failures.push(format!(
                "'{name}' puts {:.1}% of its energy above 14 kHz (limit {:.1}%) — \
                 unshaped noise reads as hiss and is what a resampler folds back",
                100.0 * share,
                100.0 * ceiling
            ));
        }
    }
    assert!(failures.is_empty(), "top end:\n  {}", failures.join("\n  "));
}
