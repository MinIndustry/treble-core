use log::info;
use std::time::Instant;

use rustic::core::envelope::prelude::{ADSREnvelopeBuilder, BezierSegment};
use rustic::core::generator::prelude::*;
use rustic::core::generator::Generator;

#[cfg(feature = "plotting")]
use rustic::plotting::plot_data;

fn main() {
    // Tone Generator
    let mut generator: Box<dyn Generator> = Box::from(
        builder::ToneGeneratorBuilder::new()
            .waveform(Waveform::Sine)
            .amplitude_envelope(Box::new(
                ADSREnvelopeBuilder::new()
                    .attack(Box::new(BezierSegment::new(0.0, 1.0, 0.2, (0.0, 1.0))))
                    .decay(Box::new(BezierSegment::new(1.0, 0.6, 0.2, (0.0, 0.6))))
                    .release(Box::new(BezierSegment::new(0.6, 0.0, 0.4, (0.4, 0.6))))
                    .build(),
            ))
            .frequency(440.0)
            .build(),
    );

    let mut generator2: Box<dyn Generator> = Box::from(
        builder::ToneGeneratorBuilder::new()
            .waveform(Waveform::Sine)
            .amplitude_envelope(Box::new(
                ADSREnvelopeBuilder::new()
                    .attack(Box::new(BezierSegment::new(0.0, 1.0, 0.1, (0.0, 1.0))))
                    .decay(Box::new(BezierSegment::new(1.0, 0.8, 0.3, (0.0, 0.6))))
                    .release(Box::new(BezierSegment::new(0.8, 0.0, 0.4, (0.4, 0.6))))
                    .build(),
            ))
            .frequency(2.0)
            .build(),
    );

    let mut results: Vec<(f32, f32)> = Vec::new();
    let mut results2: Vec<(f32, f32)> = Vec::new();
    let mut results_combined: Vec<(f32, f32)> = Vec::new();

    let sample_rate = 44100.0; // Hertz
    let duration = 1.0; // Seconds

    info!("Generating one second sample");
    let now = Instant::now();
    for sample in 0..(duration * sample_rate) as i32 {
        let current_time = sample as f32 / sample_rate;

        let val = generator.tick(1.0 / sample_rate as f32);
        let val2 = generator2.tick(1.0 / sample_rate as f32);

        results.push((current_time, val));
        results2.push((current_time, val2));
        results_combined.push((current_time, val + val2));
    }
    let elapsed = now.elapsed();

    info!("Completed");
    info!("Elapsed: {:.4?}", elapsed);

    #[cfg(feature = "plotting")]
    {
        use log::error;

        if let Err(e) = plot_data(
            results,
            "sine_440",
            (-0.1, 1.1),
            (-1.1, 1.1),
            "Sine_440.png",
        ) {
            error!("Error: {}", e.to_string());
        }

        if let Err(e) = plot_data(results2, "sine_20", (-0.1, 1.1), (-1.1, 1.1), "Sine_20.png") {
            error!("Error: {}", e.to_string());
        }

        if let Err(e) = plot_data(
            results_combined,
            "sine_20 + 440",
            (-0.1, 1.1),
            (-1.1, 1.1),
            "Sine_20_plus_440.png",
        ) {
            error!("Error: {}", e.to_string());
        }
    }
}
