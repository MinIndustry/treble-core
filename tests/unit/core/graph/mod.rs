//! Signal Graph Unit Tests
//! Tests for the audio signal graph system including sources, sinks, and connections

#[cfg(feature = "slow-test")]
use log::info;
use rustic::core::envelope::prelude::ConstantSegment;
use rustic::core::filters::prelude::*;
use rustic::core::generator::prelude::MultiToneGenerator;
use rustic::core::generator::prelude::{Waveform, builder::ToneGeneratorBuilder};
use rustic::core::graph::{SimpleSink, System, simple_source};

#[test]
fn test_system() {
    let filter = GainFilter::new(0.5);
    let filter2 = GainFilter::new(0.5);

    let mut system = System::new();
    let filt_1 = system.add_filter(Box::from(filter));
    let filt_2 = system.add_filter(Box::from(filter2));

    let mut generator: MultiToneGenerator = ToneGeneratorBuilder::new()
        .waveform(Waveform::Blank)
        .amplitude_envelope(Box::new(ConstantSegment::new(1.0, None)))
        .build()
        .into();
    generator.start();

    let source_id = system.add_source(simple_source(generator));
    system.start_source(source_id);

    let sink = SimpleSink::new();
    let sink_id = system.add_sink(Box::new(sink));

    system.connect(filt_1, filt_2, 0, 0);
    system.connect_source(source_id, filt_1, 0);
    system.connect_sink(filt_2, sink_id, 0);

    if system.compute().is_err() {
        panic!("Error computing system");
    }

    let mut results = Vec::new();
    loop {
        system.run();
        if let Ok(sink) = system.get_sink(0) {
            results.extend(sink.consume(1));
            if results.len() >= 50 {
                break;
            }
        }
    }

    for val in results.iter() {
        assert_eq!(*val, 0.25, "Values do not match !");
    }
}

#[test]
#[cfg(feature = "slow-test")]
/// Stress test of the system,
/// Testing that the system runs at least as fast as the sample rate for a simple system
fn stress_test() {
    let filter = GainFilter::new(0.5);
    let filter2 = GainFilter::new(0.5);

    let mut system = System::new();
    let filt_1 = system.add_filter(Box::from(filter));
    let filt_2 = system.add_filter(Box::from(filter2));

    let source_id = system.add_source(simple_source(
        ToneGeneratorBuilder::new()
            .waveform(Waveform::Blank)
            .amplitude_envelope(Box::new(ConstantSegment::new(1.0, None)))
            .build()
            .into(),
    ));

    let sink_id = system.add_sink(Box::new(SimpleSink::new()));

    system.connect(filt_1, filt_2, 0, 0);
    system.connect_source(source_id, filt_1, 0);
    system.connect_sink(filt_2, sink_id, 0);

    if system.compute().is_err() {
        panic!("Error computing system");
    }

    for sample_size in [100_000, 1_000_000, 10_000_000] {
        info!("Working on sample size {} at 44100 samples/s", sample_size);
        let start = std::time::Instant::now();
        for _ in 0..sample_size {
            system.run();
            if let Ok(sink) = system.get_sink(0) {
                let _ = sink.consume(1);
            }
        }
        let elapsed = start.elapsed();
        info!("Took {}ms", elapsed.as_millis());

        assert!(
            elapsed.as_millis() < ((sample_size as f32 / 44100.0) * 1000.0) as u128,
            "Test went over time !"
        );
    }
}

#[test]
#[cfg(feature = "slow-test")]
/// Stress test of the system,
/// Testing that the system runs at least as fast as the sample rate for a
/// complex system
fn stress_test_2() {
    let mut system = System::new();

    let filter_0 = CombinatorFilter::new(1, 2);
    let filter_1 = GainFilter::new(0.5);
    let filter2 = GainFilter::new(0.5);
    let filter_3 = CombinatorFilter::new(2, 1);

    let filt_0 = system.add_filter(Box::from(filter_0));
    let filt_1 = system.add_filter(Box::from(filter_1));
    let filt_2 = system.add_filter(Box::from(filter2));
    let filt_3 = system.add_filter(Box::from(filter_3));

    system.connect(filt_0, filt_1, 0, 0);
    system.connect(filt_0, filt_2, 1, 0);

    system.connect(filt_1, filt_3, 0, 0);
    system.connect(filt_2, filt_3, 0, 1);

    let source = simple_source(
        ToneGeneratorBuilder::new()
            .waveform(Waveform::Blank)
            .amplitude_envelope(Box::new(ConstantSegment::new(1.0, None)))
            .build()
            .into(),
    );
    let source_id = system.add_source(source);
    let sink_id = system.add_sink(Box::new(SimpleSink::new()));

    system.connect_source(source_id, filt_0, 0);
    system.connect_sink(filt_3, sink_id, 0);

    if system.compute().is_err() {
        panic!("Error computing system");
    }

    for sample_size in [100_000, 1_000_000, 10_000_000] {
        info!("Working on sample size {} at 44100 samples/s", sample_size);
        let start = std::time::Instant::now();
        for _ in 0..sample_size {
            system.run();
            if let Ok(sink) = system.get_sink(0) {
                let _ = sink.consume(1);
            }
        }
        let elapsed = start.elapsed();
        info!("Took {}ms", elapsed.as_millis());

        assert!(
            elapsed.as_millis() < ((sample_size as f32 / 44100.0) * 1000.0) as u128,
            "Test went over time !"
        );
    }
}

#[cfg(test)]
mod source_tests {
    // TODO: Add tests for Source trait implementations
    // - Test source initialization
    // - Test sample generation
    // - Test source chaining
}

#[cfg(test)]
mod sink_tests {
    // TODO: Add tests for Sink trait implementations
    // - Test sink consumption
    // - Test buffer management
    // - Test multiple sinks
}

#[cfg(test)]
mod system_tests {
    // TODO: Add more System tests
    // - Test graph validation
    // - Test cycle detection
    // - Test disconnection
    // - Test dynamic reconfiguration
}
