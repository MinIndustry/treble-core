use rustic::core::envelope::prelude::{ADSREnvelopeBuilder, BezierSegment};

use rustic::prelude::App;

fn main() {
    let app = App::init();
    let scale = app.config.system.master_volume;
    let sample_rate = app.config.system.sample_rate;

    let envelope = ADSREnvelopeBuilder::new()
        .attack(Box::new(BezierSegment::new(0.0, scale * 1.0, 0.1, (0.1, 0.0))))
        .decay(Box::new(BezierSegment::new(scale * 1.0, scale * 0.2, 0.4, (0.5, scale * 1.0))))
        .release(Box::new(BezierSegment::new(scale * 0.2, 0.0, 0.5, (0.5, 0.0))))
        .build();

    // This example needs to be updated to work with the new architecture
    // For now, we'll just print the configuration
    println!("Sample rate: {}", sample_rate);
    println!("Master volume: {}", scale);
    println!("This example needs to be updated for the new core structure");
}
