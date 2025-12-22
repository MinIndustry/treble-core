use rustic::core::envelope::prelude::ADSREnvelope;

use rustic::prelude::App;

fn main() {
    let app = App::init();
    let scale = app.config.system.master_volume;
    let sample_rate = app.config.system.sample_rate;

    let envelope = {
        let mut env = ADSREnvelope::new();
        env.set_attack(0.1, scale * 1.0, Some((0.1, 0.0)));
        env.set_decay(0.4, scale * 0.2, Some((0.5, scale * 1.0)));
        env.set_release(0.5, scale * 0.0, Some((0.5, 0.0)));
        env
    };

    // This example needs to be updated to work with the new architecture
    // For now, we'll just print the configuration
    println!("Sample rate: {}", sample_rate);
    println!("Master volume: {}", scale);
    println!("This example needs to be updated for the new core structure");
}
