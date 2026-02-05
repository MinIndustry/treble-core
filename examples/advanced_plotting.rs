//! Advanced plotting features demonstration
//!
//! This example demonstrates the advanced features of the plotting module:
//! - Multiple data series with custom colors
//! - Line annotations (vertical, horizontal, and custom)
//! - Custom resolution and styling
//! - Axis labels
//! - Legend and grid configuration

#[cfg(not(feature = "plotting"))]
fn main() -> Result<(), ()> {
    println!("Can't run example without the \"plotting\" feature");

    Ok(())
}

#[cfg(feature = "plotting")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use rustic::plotting::PlotBuilder;
    use std::f32::consts::PI;

    println!("Generating advanced plot with multiple series and annotations...");

    // Generate sample data: sine and cosine waves
    let sine: Vec<(f32, f32)> = (0..200)
        .map(|i| {
            let x = i as f32 / 200.0 * 2.0 * PI;
            (x, x.sin())
        })
        .collect();

    let cosine: Vec<(f32, f32)> = (0..200)
        .map(|i| {
            let x = i as f32 / 200.0 * 2.0 * PI;
            (x, x.cos())
        })
        .collect();

    // Create an advanced plot with full customization
    PlotBuilder::new()
        .title("Trigonometric Functions")
        .x_label("Angle (radians)")
        .y_label("Amplitude")
        .x_range(0.0, 2.0 * PI)
        .y_range(-1.5, 1.5)
        // Add data series with custom colors
        .add_series(sine, "sin(x)", Some((255, 0, 0))) // Red
        .add_series(cosine, "cos(x)", Some((0, 0, 255))) // Blue
        // Add reference lines
        .add_horizontal_line(0.0, Some((128, 128, 128))) // Gray zero line
        .add_vertical_line(PI, Some((0, 128, 0))) // Green line at π
        .add_vertical_line(PI / 2.0, Some((200, 200, 200))) // Light gray at π/2
        // Customize appearance
        .resolution(1600, 900)
        .font_family("sans-serif")
        .title_font_size(40)
        .label_font_size(28)
        .show_legend(true)
        .show_grid(true)
        .save("advanced_plot.png")?;

    println!("✓ Plot saved to advanced_plot.png");

    // Create a second example with custom line annotation
    println!("\nGenerating plot with custom diagonal line...");

    let data: Vec<(f32, f32)> = (0..100)
        .map(|i| {
            let x = i as f32 / 100.0;
            (x, x * x) // Parabola
        })
        .collect();

    PlotBuilder::new()
        .title("Parabola with Tangent Line")
        .x_label("x")
        .y_label("y = x²")
        .x_range(0.0, 1.0)
        .y_range(0.0, 1.0)
        .add_series(data, "y = x²", Some((255, 127, 0))) // Orange
        // Add a custom diagonal line (tangent at x=0.5)
        .add_line((0.0, -0.5), (1.0, 1.5), Some((0, 128, 0)))
        .resolution(1280, 720)
        .save("parabola_tangent.png")?;

    println!("✓ Plot saved to parabola_tangent.png");

    // Create a third example showing minimal configuration
    println!("\nGenerating simple plot with defaults...");

    use rustic::plotting::plot_data;

    let simple_data: Vec<(f32, f32)> = (0..50)
        .map(|i| {
            let x = i as f32 / 50.0;
            (x, (x * 10.0).sin())
        })
        .collect();

    plot_data(
        simple_data,
        "Simple Oscillation",
        (0.0, 1.0),
        (-1.0, 1.0),
        "simple_oscillation.png",
    )?;

    Ok(())
}
