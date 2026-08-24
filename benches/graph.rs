use criterion::{Criterion, black_box, criterion_group, criterion_main};

use treble::core::{
    filters::prelude::*,
    generator::prelude::{
        FrequencyRelation, Waveform,
        builder::{MultiToneGeneratorBuilder, ToneGeneratorBuilder},
    },
    graph::{SimpleSink, System, simple_source},
};

const SAMPLE_RATE: f32 = 44100.0;
const BLOCK_SIZE: usize = 512;

// Each bench iteration renders this many blocks (×BLOCK_SIZE = total frames).
// SIMPLE: 4096 blocks × 512 = ~2M frames  ≈ 47 s of audio rendered in one iter.
// COMPLEX: 1024 blocks × 512 = ~500k frames ≈ 12 s of audio rendered in one iter.
const SIMPLE_TICKS: usize = 4096;
const COMPLEX_TICKS: usize = 1024;

// ---------------------------------------------------------------------------
// Graph builders
// ---------------------------------------------------------------------------

/// Simple linear graph: 1 active source → LowPass → Gain → LowPass → Sink.
/// No feedback, no branching. Baseline for raw throughput.
fn build_simple_system() -> System {
    let mut system = System::new().with_block_size(BLOCK_SIZE);

    let tone_gen = MultiToneGeneratorBuilder::new()
        .frequency(440.0)
        .add_generator(
            ToneGeneratorBuilder::new()
                .waveform(Waveform::Sine)
                .frequency_relation(FrequencyRelation::Identity)
                .build(),
        )
        .build();
    let src = system.add_source(simple_source(tone_gen));

    let lp1 = system.add_filter(Box::new(LowPassFilter::new(2000.0, SAMPLE_RATE)));
    let gain = system.add_filter(Box::new(GainFilter::new(0.8)));
    let lp2 = system.add_filter(Box::new(LowPassFilter::new(1000.0, SAMPLE_RATE)));

    system.connect_source(src, lp1, 0);
    system.connect(lp1, gain, 0, 0);
    system.connect(gain, lp2, 0, 0);

    let sink = system.add_sink(Box::new(SimpleSink::new()));
    system.connect_sink(lp2, sink, 0);

    system.compute().unwrap();
    system.start_source(0);
    system
}

/// Complex graph: 4 active sources, 12+ filter nodes, 1 feedback delay loop.
///
/// Layout:
///   src1 (sine  440) → LowPass → sum_node → Delay(0.2s) → att(0.4) (Feedback to sum node)
///                       sum_node → master
///   src2 (square 880) → HighPass → Tremolo → master
///   src3 (saw   220)  → LowPass → LowPass  → master
///   src4 (noise)      → LowPass → LowPass  → master
///   master(×0.25) → Compressor → FinalGain(×0.8) → Sink
fn build_complex_system() -> System {
    let mut system = System::new().with_block_size(BLOCK_SIZE);

    // --- source 1: sine 440 Hz with feedback delay loop ---
    let src1 = system.add_source(simple_source(
        MultiToneGeneratorBuilder::new()
            .frequency(440.0)
            .add_generator(
                ToneGeneratorBuilder::new()
                    .waveform(Waveform::Sine)
                    .frequency_relation(FrequencyRelation::Identity)
                    .build(),
            )
            .build(),
    ));

    let lp1 = system.add_filter(Box::new(LowPassFilter::new(2000.0, SAMPLE_RATE)));
    // sum_node receives both the filtered source and the attenuated feedback.
    // With the default MixMode::Sum, both port-0 pushes are summed automatically.
    let sum_node = system.add_filter(Box::new(GainFilter::new(1.0)));
    let delay = system.add_filter(Box::new(DelayFilter::new(SAMPLE_RATE, 0.2)));
    let att = system.add_filter(Box::new(GainFilter::new(0.4)));

    system.connect_source(src1, lp1, 0);
    system.connect(lp1, sum_node, 0, 0); // forward path
    system.connect(sum_node, delay, 0, 0); // into delay
    system.connect(delay, att, 0, 0); // attenuate
    system.connect(att, sum_node, 0, 0); // feedback → same port 0, summed

    // --- source 2: square 880 Hz → HighPass → Tremolo ---
    let src2 = system.add_source(simple_source(
        MultiToneGeneratorBuilder::new()
            .frequency(880.0)
            .add_generator(
                ToneGeneratorBuilder::new()
                    .waveform(Waveform::Square)
                    .frequency_relation(FrequencyRelation::Identity)
                    .build(),
            )
            .build(),
    ));

    let hp = system.add_filter(Box::new(HighPassFilter::new(500.0, SAMPLE_RATE)));
    let tremolo = system.add_filter(Box::new(Tremolo::new(6.0, 0.5, SAMPLE_RATE)));

    system.connect_source(src2, hp, 0);
    system.connect(hp, tremolo, 0, 0);

    // --- source 3: sawtooth 220 Hz → 2× LowPass ---
    let src3 = system.add_source(simple_source(
        MultiToneGeneratorBuilder::new()
            .frequency(220.0)
            .add_generator(
                ToneGeneratorBuilder::new()
                    .waveform(Waveform::Sawtooth)
                    .frequency_relation(FrequencyRelation::Identity)
                    .build(),
            )
            .build(),
    ));

    let lp2 = system.add_filter(Box::new(LowPassFilter::new(1500.0, SAMPLE_RATE)));
    let lp3 = system.add_filter(Box::new(LowPassFilter::new(800.0, SAMPLE_RATE)));

    system.connect_source(src3, lp2, 0);
    system.connect(lp2, lp3, 0, 0);

    // --- source 4: white noise → 2× heavy LowPass ---
    let src4 = system.add_source(simple_source(
        MultiToneGeneratorBuilder::new()
            .add_generator(
                ToneGeneratorBuilder::new()
                    .waveform(Waveform::WhiteNoise)
                    .build(),
            )
            .build(),
    ));

    let lp4 = system.add_filter(Box::new(LowPassFilter::new(400.0, SAMPLE_RATE)));
    let lp5 = system.add_filter(Box::new(LowPassFilter::new(200.0, SAMPLE_RATE)));

    system.connect_source(src4, lp4, 0);
    system.connect(lp4, lp5, 0, 0);

    // --- master: all paths fan-in on port 0 (summed), then compress + gain ---
    let master = system.add_filter(Box::new(GainFilter::new(0.25)));
    let compressor = system.add_filter(Box::new(Compressor::default()));
    let final_gain = system.add_filter(Box::new(GainFilter::new(0.8)));

    system.connect(sum_node, master, 0, 0); // src1 path
    system.connect(tremolo, master, 0, 0); // src2 path
    system.connect(lp3, master, 0, 0); // src3 path
    system.connect(lp5, master, 0, 0); // src4 path

    system.connect(master, compressor, 0, 0);
    system.connect(compressor, final_gain, 0, 0);

    let sink = system.add_sink(Box::new(SimpleSink::new()));
    system.connect_sink(final_gain, sink, 0);

    system.compute().unwrap();

    for i in 0..4 {
        system.start_source(i);
    }
    system
}

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

fn bench_simple_graph(c: &mut Criterion) {
    let mut system = build_simple_system();

    c.bench_function("simple_graph", |b| {
        b.iter(|| {
            for _ in 0..SIMPLE_TICKS {
                system.run();
                // Drain sink to prevent unbounded accumulation.
                let _ = system.get_sink(0).map(|s| black_box(s.consume()));
            }
        })
    });
}

fn bench_complex_graph(c: &mut Criterion) {
    let mut system = build_complex_system();

    c.bench_function("complex_graph", |b| {
        b.iter(|| {
            for _ in 0..COMPLEX_TICKS {
                system.run();
                let _ = system.get_sink(0).map(|s| black_box(s.consume()));
            }
        })
    });
}

/// A realistic Treble Live performance: six built-in instruments compiled
/// through the production path (`compile_spec`), several notes held on each.
/// This is what the render thread actually pays for per block.
fn bench_session(c: &mut Criterion) {
    use treble::core::utils::Note;
    use treble::instruments::registry::InstrumentRegistry;
    use treble::instruments::spec::compile_spec;

    let registry = InstrumentRegistry::built_in();
    let names = ["kick", "snare", "hihat", "bass", "pluck", "pad"];
    let mut systems: Vec<System> = names
        .iter()
        .map(|name| {
            let spec = registry.get(name).expect("built-in exists");
            let mut system = compile_spec(spec, SAMPLE_RATE).expect("built-in spec compiles");
            // Hold a small chord per instrument so poly voices are live.
            for midi in [48u8, 55, 60] {
                system.start_note(0, Note::from_midi(midi), 0.9);
            }
            system
        })
        .collect();

    c.bench_function("session_6_instruments", |b| {
        b.iter(|| {
            // 256 blocks x 512 frames = ~3s of audio per iteration.
            for _ in 0..256 {
                for system in systems.iter_mut() {
                    system.run();
                    let _ = system.get_sink(0).map(|s| black_box(s.consume()));
                }
            }
        })
    });
}

/// Per-block filter cost: the shipped Gain/Pan filters against inline serial
/// closures on the same 512-frame block. Historical note: the shipped versions
/// used rayon `par_iter` until 2026-08-24, measured here at ~1,300x slower
/// than serial (125 us vs 96 ns) — keep these benches so a regression shows.
fn bench_filter_parallelism(c: &mut Criterion) {
    use std::sync::Arc;
    use treble::core::Block;
    use treble::core::graph::{Entry, Filter};

    let block: Arc<Block> = Arc::new(vec![[0.25f32, -0.25f32]; BLOCK_SIZE]);

    let mut gain = GainFilter::new(0.8);
    c.bench_function("gain_filter_shipped", |b| {
        b.iter(|| {
            gain.push(Arc::clone(&block), 0);
            black_box(gain.transform())
        })
    });

    let mut pan = PanFilter::new(-0.3);
    c.bench_function("pan_filter_shipped", |b| {
        b.iter(|| {
            pan.push(Arc::clone(&block), 0);
            black_box(pan.transform())
        })
    });

    c.bench_function("gain_serial_equivalent", |b| {
        b.iter(|| {
            let out: Block = block.iter().map(|[l, r]| [l * 0.8, r * 0.8]).collect();
            black_box(out)
        })
    });

    c.bench_function("pan_serial_equivalent", |b| {
        let (lg, rg) = ((1.0 - (-0.3f32)) * 0.5, (1.0 + (-0.3f32)) * 0.5);
        b.iter(|| {
            let out: Block = block.iter().map(|[l, r]| [l * lg, r * rg]).collect();
            black_box(out)
        })
    });
}

criterion_group!(
    benches,
    bench_simple_graph,
    bench_complex_graph,
    bench_session,
    bench_filter_parallelism
);
criterion_main!(benches);
