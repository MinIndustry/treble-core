//! Graph / System Unit Tests

mod sources;

use treble::core::audio::{Block, CHANNELS};
use treble::core::filters::prelude::{DelayFilter, GainFilter};
use treble::core::graph::{ModTarget, SimpleSink, Source, System};

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
    fn unknown_runtime_parameters_are_rejected() {
        let mut system = System::new();
        let source = system.add_source(Box::new(ConstantSource { value: 0.5 }));
        let filter = system.add_filter(Box::new(GainFilter::new(1.0)));

        let source_error = system
            .set_source_parameter(source, "amplitdue", 0.5)
            .expect_err("unknown source parameter should fail");
        assert!(source_error.to_string().contains("amplitdue"));

        let modulation_error = system
            .add_mod_wire(source, ModTarget::Filter(filter), "gaim".into())
            .expect_err("unknown modulation target parameter should fail");
        assert!(modulation_error.to_string().contains("gaim"));
    }

    #[test]
    fn test_system_run_basic() {
        let mut system = build_simple_system(2.0, 16);
        system.run();

        let stats = system.last_run_stats();
        assert_eq!(stats.frames, 16);
        assert_eq!(stats.processed_nodes, 1);
        assert_eq!(stats.source_routes, 1);
        assert_eq!(stats.filter_routes, 0);
        assert_eq!(stats.sink_routes, 1);

        let sink = system.get_sink(0).unwrap();
        let frames = sink.consume();
        assert_eq!(frames.len(), 16, "Should produce exactly block_size frames");
        for frame in &frames {
            assert!(
                (frame[0] - 1.0).abs() < 1e-5,
                "0.5 * gain(2.0) = 1.0, got {}",
                frame[0]
            );
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
            assert!(
                (frame[0] - 0.5).abs() < 1e-5,
                "L: expected 0.5, got {}",
                frame[0]
            );
            assert!(
                (frame[1] - 1.5).abs() < 1e-5,
                "R: expected 1.5, got {}",
                frame[1]
            );
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
            assert!(
                (frame[0] - 8.0).abs() < 1e-4,
                "Expected 8.0, got {}",
                frame[0]
            );
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
        assert!(
            result.is_err(),
            "Cycle without postponable filter should error"
        );
    }

    #[test]
    fn test_system_cycle_broken_by_delay() {
        // DelayFilter is postponable=true, so it breaks the cycle.
        // Two signals arrive at port 0 of a GainFilter (source + delayed feedback).
        let mut system = System::new().with_block_size(8);
        let mixer = system.add_filter(Box::new(GainFilter::new(1.0)));
        let gain = system.add_filter(Box::new(GainFilter::new(0.5)));
        let delay = system.add_filter(Box::new(DelayFilter::new(44100.0, 0.001)));

        let src = system.add_source(Box::new(ConstantSource { value: 1.0 }));
        let snk = system.add_sink(Box::new(SimpleSink::new()));

        // source → mixer (port 0); delayed feedback also → mixer (port 0)
        system.connect_source(src, mixer, 0);
        system.connect(mixer, gain, 0, 0);
        system.connect(gain, delay, 0, 0);
        system.connect(delay, mixer, 0, 0); // feedback on same port — DelayFilter breaks cycle
        system.connect_sink(gain, snk, 0);

        // Should succeed because DelayFilter is postponable
        let result = system.compute();
        assert!(
            result.is_ok(),
            "Cycle with delay should succeed: {:?}",
            result
        );
    }

    #[test]
    fn test_system_two_sources_combined() {
        // Two sources connect to port 0 of a GainFilter; the run loop sums them.
        let mut system = System::new().with_block_size(4);
        let mixer = system.add_filter(Box::new(GainFilter::new(1.0)));
        let src1 = system.add_source(Box::new(ConstantSource { value: 0.3 }));
        let src2 = system.add_source(Box::new(ConstantSource { value: 0.7 }));
        let snk = system.add_sink(Box::new(SimpleSink::new()));

        system.connect_source(src1, mixer, 0);
        system.connect_source(src2, mixer, 0);
        system.connect_sink(mixer, snk, 0);
        system.compute().unwrap();
        system.run();

        let frames = system.get_sink(0).unwrap().consume();
        for frame in &frames {
            assert!(
                (frame[0] - 1.0).abs() < 1e-5,
                "0.3+0.7=1.0, got {}",
                frame[0]
            );
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

#[cfg(test)]
mod postponable_layering_tests {
    use super::*;
    use treble::core::filters::prelude::ReverbFilter;

    /// A mid-chain postponable filter runs *after* its feeder in one run.
    ///
    /// It did not: every edge into a postponable node was dropped before the
    /// topological sort, so `source → gain → reverb → sink` put the reverb in
    /// layer 0, one whole run behind the gain. At a fixed block size that was
    /// mere latency, but a run split at a note event handed the sink a block
    /// of the previous run's length — and the resize dropped samples, which
    /// was an audible click on nearly every note boundary of a rendered piece.
    #[test]
    fn a_mid_chain_postponable_filter_is_not_a_run_behind() {
        let mut system = System::new().with_block_size(16);
        let gain = system.add_filter(Box::new(GainFilter::new(1.0)));
        let reverb = system.add_filter(Box::new(ReverbFilter::new(44_100.0, 0.5)));
        let source = system.add_source(Box::new(ConstantSource { value: 0.5 }));
        let sink = system.add_sink(Box::new(SimpleSink::new()));

        system.connect_source(source, gain, 0);
        system.connect(gain, reverb, 0, 0);
        system.connect_sink(reverb, sink, 0);
        system.compute().expect("a straight chain must compute");

        // The dry path passes through the reverb immediately, so the very
        // first run must already carry the signal at full length.
        system.run_frames(16);
        let frames = system.get_sink(0).unwrap().consume();
        assert_eq!(frames.len(), 16, "the first run lost its block");
        assert!(
            (frames[0][0] - 0.25).abs() < 1e-5,
            "expected the dry half of 0.5 on the first sample, got {}",
            frames[0][0]
        );

        // Split runs — what note events do to a block — must each produce
        // exactly their own length, not the previous run's.
        for split in [7usize, 9, 1, 15] {
            system.run_frames(split);
            let frames = system.get_sink(0).unwrap().consume();
            assert_eq!(
                frames.len(),
                split,
                "a split run came back the wrong length"
            );
        }
    }

    /// A genuine feedback loop through a postponable filter still compiles:
    /// only the cycle-closing edge is postponed, not every edge into it.
    #[test]
    fn a_feedback_loop_through_a_postponable_filter_still_computes() {
        let mut system = System::new().with_block_size(16);
        let gain = system.add_filter(Box::new(GainFilter::new(0.5)));
        let delay = system.add_filter(Box::new(DelayFilter::default()));
        let source = system.add_source(Box::new(ConstantSource { value: 0.5 }));
        let sink = system.add_sink(Box::new(SimpleSink::new()));

        system.connect_source(source, gain, 0);
        system.connect(gain, delay, 0, 0);
        system.connect(delay, gain, 0, 1);
        system.connect_sink(gain, sink, 0);
        system
            .compute()
            .expect("a delay-broken cycle must still compute");
    }
}
