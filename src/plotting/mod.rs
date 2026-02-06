//! Plotting utilities for visualizing waveforms and data
//!
//! This module provides a two-tier API for creating plots:
//! 1. **Simple convenience functions** for quick plots with sensible defaults
//! 2. **Builder pattern** for full customization
//!
//! # Quick Start
//!
//! For simple single-series plots, use the `plot_data` function:
//!
//! ```
//! use rustic::plotting::plot_data;
//!
//! let data = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 4.0)];
//! plot_data(data, "My Plot", (0.0, 2.0), (0.0, 5.0), "output.png")?;
//! # Ok::<(), rustic::plotting::PlotError>(())
//! ```
//!
//! For multiple series, use `plot_multi`:
//!
//! ```
//! use rustic::plotting::plot_multi;
//!
//! let series = vec![
//!     (vec![(0.0, 0.0), (1.0, 1.0)], "Series 1"),
//!     (vec![(0.0, 1.0), (1.0, 0.0)], "Series 2"),
//! ];
//! plot_multi(series, "Multi Plot", (0.0, 1.0), (0.0, 1.0), "output.png")?;
//! # Ok::<(), rustic::plotting::PlotError>(())
//! ```
//!
//! # Advanced Usage
//!
//! For full customization, use [`PlotBuilder`]:
//!
//! ```
//! use rustic::plotting::PlotBuilder;
//!
//! let data = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 4.0)];
//!
//! PlotBuilder::new()
//!     .title("Advanced Plot")
//!     .x_label("Time (s)")
//!     .y_label("Amplitude")
//!     .x_range(0.0, 2.0)
//!     .y_range(0.0, 5.0)
//!     .add_series(data, "Data", Some((255, 0, 0)))  // Red color
//!     .add_horizontal_line(2.0, None)                // Gray reference line
//!     .resolution(1280, 720)
//!     .show_legend(true)
//!     .save("advanced.png")?;
//! # Ok::<(), rustic::plotting::PlotError>(())
//! ```

// Internal modules
mod builder;
mod error;
mod line;
mod render;
mod serie;
mod types;

// Public exports
pub use builder::PlotBuilder;
pub use error::PlotError;

// Internal types (not re-exported)
#[allow(unused_imports)]
use line::Line;
#[allow(unused_imports)]
use serie::PlotSerie;
#[allow(unused_imports)]
use types::{LineConfig, LineType, SeriesConfig};

/// Prelude module for convenient imports
pub mod prelude {
    //! Convenient imports for plotting
    //!
    //! Use `use rustic::plotting::prelude::*;` to import commonly used types.
    pub use super::{PlotBuilder, PlotError, plot_data, plot_multi};
}

// ==================== Convenience Functions ====================

/// Plot a single data series with automatic styling
///
/// This is the simplest way to create a plot. It generates a plot with sensible
/// defaults (1920x1080 resolution, white background, grid enabled, legend shown).
///
/// # Arguments
/// * `data` - Vector of (x, y) coordinate pairs
/// * `title` - Plot title
/// * `x_scale` - X-axis range as (min, max)
/// * `y_scale` - Y-axis range as (min, max)
/// * `filename` - Output file path (typically .png)
///
/// # Example
/// ```
/// use rustic::plotting::plot_data;
///
/// let data: Vec<(f32, f32)> = (0..100)
///     .map(|i| {
///         let x = i as f32 / 100.0;
///         (x, x.sin())
///     })
///     .collect();
///
/// plot_data(data, "Sine Wave", (0.0, 1.0), (-1.0, 1.0), "sine.png")?;
/// # Ok::<(), rustic::plotting::PlotError>(())
/// ```
///
/// # Errors
/// Returns [`PlotError`] if:
/// - Data is empty
/// - Invalid axis ranges (min >= max)
/// - File cannot be written
/// - Rendering fails
pub fn plot_data(
    data: Vec<(f32, f32)>,
    title: &str,
    x_scale: (f32, f32),
    y_scale: (f32, f32),
    filename: &str,
) -> Result<(), PlotError> {
    PlotBuilder::new()
        .title(title)
        .x_range(x_scale.0, x_scale.1)
        .y_range(y_scale.0, y_scale.1)
        .add_series(data, title, None)
        .save(filename)
}

/// Plot multiple data series with automatic color assignment
///
/// Creates a plot with multiple data series, automatically assigning distinct
/// colors to each series from a 10-color palette.
///
/// # Arguments
/// * `series` - Vector of (data, label) tuples where:
///   - `data` is a Vec<(f32, f32)> of coordinate pairs
///   - `label` is the series name (shown in legend)
/// * `title` - Plot title
/// * `x_scale` - X-axis range as (min, max)
/// * `y_scale` - Y-axis range as (min, max)
/// * `filename` - Output file path (typically .png)
///
/// # Example
/// ```
/// use rustic::plotting::plot_multi;
///
/// let sine: Vec<(f32, f32)> = (0..100)
///     .map(|i| {
///         let x = i as f32 / 100.0;
///         (x, x.sin())
///     })
///     .collect();
///
/// let cosine: Vec<(f32, f32)> = (0..100)
///     .map(|i| {
///         let x = i as f32 / 100.0;
///         (x, x.cos())
///     })
///     .collect();
///
/// plot_multi(
///     vec![(sine, "sin(x)"), (cosine, "cos(x)")],
///     "Trig Functions",
///     (0.0, 1.0),
///     (-1.0, 1.0),
///     "trig.png"
/// )?;
/// # Ok::<(), rustic::plotting::PlotError>(())
/// ```
///
/// # Errors
/// Returns [`PlotError`] if:
/// - No series provided or all series are empty
/// - Invalid axis ranges (min >= max)
/// - File cannot be written
/// - Rendering fails
pub fn plot_multi<S: AsRef<str>>(
    series: Vec<(Vec<(f32, f32)>, S)>,
    title: &str,
    x_scale: (f32, f32),
    y_scale: (f32, f32),
    filename: &str,
) -> Result<(), PlotError> {
    let mut builder = PlotBuilder::new()
        .title(title)
        .x_range(x_scale.0, x_scale.1)
        .y_range(y_scale.0, y_scale.1);

    for (data, label) in series {
        builder = builder.add_series(data, label.as_ref(), None);
    }

    builder.save(filename)
}
