//! Simple testing GUI for the rustic audio system
//!
//! This example provides a basic GUI for testing audio functionality without
//! requiring the full frontend. It demonstrates:
//! - Starting and stopping notes
//! - Adjusting volume
//! - Changing octaves
//! - Monitoring system metrics
//! - Viewing backend events
//!
//! ## Usage
//!
//! Run this example with the testing feature enabled:
//!
//! ```bash
//! cargo run --example testing_gui --features testing
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "testing")]
    return rustic::testing::run_testing_gui();

    #[cfg(not(feature = "testing"))]
    {
        eprintln!("Error: The 'testing' feature is not enabled.");
        eprintln!();
        eprintln!("To run this example, use:");
        eprintln!("  cargo run --example testing_gui --features testing");
        std::process::exit(1);
    }
}
