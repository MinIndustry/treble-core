//! Graph / System Unit Tests

use rustic::core::audio::{Block, CHANNELS};
use rustic::core::filters::prelude::{CombinatorFilter, DelayFilter, GainFilter};
use rustic::core::graph::{SimpleSink, Source, System};

/// A trivial source that emits a constant stereo block.
#[derive(Debug, Clone)]
struct ConstantSource {
    value: f32,
}

impl Source for ConstantSource {
    fn pull(&mut self, block_size: usize) -> Block {
        vec![[self.value; CHANNELS]; block_size]
    }
}

/// Helper: build a minimal system: source → gain → sink.
fn build_simple_system(gain: f32, block_size: usize) -> System {
    let mut system = System::new().with_block_size(block_size);

    let gain_filter = system.add_filter(Box::new(GainFilter::new(gain)));
    let source_idx = system.add_source(Box::new(ConstantSource { value: 0.5 }));
    let sink_idx = system.add_sink(Box::new(SimpleSink::new()));

    system.connect_source(source_idx, gain_filter, 0);
    system.connect_sink(gain_filter, sink_idx, 0);
    system.compute().expect("compute should succeed");

    system
}

#[cfg(test)]
mod system_tests {
    use super::*;

    #[test]
    fn test_system_run_basic() {
        let mut system = build_simple_system(2.0, 16);
        system.run();

        let sink = system.get_sink(0).unwrap();
        let frames = sink.consume();
        assert_eq!(frames.len(), 16, "Should produce exactly block_size frames");
        for frame in &frames {
            assert!((frame[0] - 1.0).abs() < 1e-5, "0.5 * gain(2.0) = 1.0, got {}", frame[0]);
            assert!((frame[1] - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn test_system_block_size() {
        for block_size in [1, 16, 64, 512] {
            let mut system = build_simple_system(1.0, block_size);
            system.run();
            let sink = system.get_sink(0).unwrap();
            let frames = sink.consume();
            assert_eq!(
                frames.len(),
                block_size,
                "block_size={}: expected {} frames",
                block_size,
                block_size
            );
        }
    }

    #[test]
    fn test_system_stereo_channels_independent() {
        /// Source that emits different L and R values.
        #[derive(Debug, Clone)]
        struct StereoSource;
        impl Source for StereoSource {
            fn pull(&mut self, n: usize) -> Block {
                (0..n).map(|_| [0.25_f32, 0.75_f32]).collect()
            }
        }

        let mut system = System::new().with_block_size(8);
        let gain = system.add_filter(Box::new(GainFilter::new(2.0)));
        let src = system.add_source(Box::new(StereoSource));
        let snk = system.add_sink(Box::new(SimpleSink::new()));

        system.connect_source(src, gain, 0);
        system.connect_sink(gain, snk, 0);
        system.compute().unwrap();
        system.run();

        let frames = system.get_sink(0).unwrap().consume();
        for frame in &frames {
            assert!((frame[0] - 0.5).abs() < 1e-5, "L: expected 0.5, got {}", frame[0]);
            assert!((frame[1] - 1.5).abs() < 1e-5, "R: expected 1.5, got {}", frame[1]);
        }
    }

    #[test]
    fn test_system_compute_layers() {
        // chain: gain1 → gain2 → gain3
        let mut system = System::new().with_block_size(4);
        let g1 = system.add_filter(Box::new(GainFilter::new(2.0)));
        let g2 = system.add_filter(Box::new(GainFilter::new(2.0)));
        let g3 = system.add_filter(Box::new(GainFilter::new(2.0)));
        let src = system.add_source(Box::new(ConstantSource { value: 1.0 }));
        let snk = system.add_sink(Box::new(SimpleSink::new()));

        system.connect(g1, g2, 0, 0);
        system.connect(g2, g3, 0, 0);
        system.connect_source(src, g1, 0);
        system.connect_sink(g3, snk, 0);
        system.compute().unwrap();
        system.run();

        // 1.0 * 2 * 2 * 2 = 8.0
        let frames = system.get_sink(0).unwrap().consume();
        for frame in &frames {
            assert!((frame[0] - 8.0).abs() < 1e-4, "Expected 8.0, got {}", frame[0]);
        }
    }

    #[test]
    fn test_system_cycle_detection() {
        let mut system = System::new();
        let a = system.add_filter(Box::new(GainFilter::new(1.0)));
        let b = system.add_filter(Box::new(GainFilter::new(1.0)));
        // Create a cycle: a → b → a (no postponable filter to break it)
        system.connect(a, b, 0, 0);
        system.connect(b, a, 0, 0);
        let result = system.compute();
        assert!(result.is_err(), "Cycle without postponable filter should error");
    }

    #[test]
    fn test_system_cycle_broken_by_delay() {
        // DelayFilter is postponable=true, so it breaks the cycle
        let mut system = System::new().with_block_size(8);
        let gain = system.add_filter(Box::new(GainFilter::new(0.5)));
        let delay = system.add_filter(Box::new(DelayFilter::new(44100.0, 0.001)));
        let combinator = system.add_filter(Box::new(CombinatorFilter::new(2, 1)));

        let src = system.add_source(Box::new(ConstantSource { value: 1.0 }));
        let snk = system.add_sink(Box::new(SimpleSink::new()));

        // source → combinator → gain → delay → combinator (feedback loop)
        system.connect_source(src, combinator, 0);
        system.connect(combinator, gain, 0, 0);
        system.connect(gain, delay, 0, 0);
        system.connect(delay, combinator, 0, 1);
        system.connect_sink(gain, snk, 0);

        // Should succeed because DelayFilter is postponable
        let result = system.compute();
        assert!(result.is_ok(), "Cycle with delay should succeed: {:?}", result);
    }

    #[test]
    fn test_system_two_sources_combined() {
        let mut system = System::new().with_block_size(4);
        let combinator = system.add_filter(Box::new(CombinatorFilter::new(2, 1)));
        let src1 = system.add_source(Box::new(ConstantSource { value: 0.3 }));
        let src2 = system.add_source(Box::new(ConstantSource { value: 0.7 }));
        let snk = system.add_sink(Box::new(SimpleSink::new()));

        system.connect_source(src1, combinator, 0);
        system.connect_source(src2, combinator, 1);
        system.connect_sink(combinator, snk, 0);
        system.compute().unwrap();
        system.run();

        let frames = system.get_sink(0).unwrap().consume();
        for frame in &frames {
            assert!((frame[0] - 1.0).abs() < 1e-5, "0.3+0.7=1.0, got {}", frame[0]);
        }
    }

    #[test]
    fn test_system_with_block_size_builder() {
        let system = System::new().with_block_size(128);
        assert_eq!(system.block_size(), 128);
    }

    #[test]
    fn test_system_source_start_stop() {
        let mut system = build_simple_system(1.0, 8);
        // start/stop should not panic
        system.start_source(0);
        system.stop_source(0);
    }
}
