//! Builder pattern for creating plots with customization options

use crate::plotting::{
    types::{LineConfig, LineType, SeriesConfig},
    PlotError,
};
use std::path::Path;

/// Builder for creating customizable plots
///
/// Provides a fluent API for configuring all aspects of a plot including
/// data series, annotations, styling, and output settings.
///
/// # Example
/// ```
/// use rustic::plotting::PlotBuilder;
///
/// let data = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 4.0)];
///
/// PlotBuilder::new()
///     .title("My Plot")
///     .x_range(0.0, 2.0)
///     .y_range(0.0, 5.0)
///     .add_series(data, "Data", None)
///     .add_horizontal_line(2.0, None)
///     .resolution(1280, 720)
///     .save("output.png")?;
/// # Ok::<(), rustic::plotting::PlotError>(())
/// ```
#[derive(Debug, Clone)]
pub struct PlotBuilder {
    pub(crate) title: String,
    pub(crate) x_range: (f32, f32),
    pub(crate) y_range: (f32, f32),
    pub(crate) x_label: Option<String>,
    pub(crate) y_label: Option<String>,
    pub(crate) series: Vec<SeriesConfig>,
    pub(crate) lines: Vec<LineConfig>,
    pub(crate) resolution: (u32, u32),
    pub(crate) font_family: String,
    pub(crate) title_font_size: u32,
    pub(crate) label_font_size: u32,
    pub(crate) margin: u32,
    pub(crate) background_color: (u8, u8, u8),
    pub(crate) show_legend: bool,
    pub(crate) show_grid: bool,
}

impl Default for PlotBuilder {
    fn default() -> Self {
        Self {
            title: "Plot".to_string(),
            x_range: (0.0, 1.0),
            y_range: (0.0, 1.0),
            x_label: None,
            y_label: None,
            series: Vec::new(),
            lines: Vec::new(),
            resolution: (1920, 1080),
            font_family: "sans-serif".to_string(),
            title_font_size: 50,
            label_font_size: 30,
            margin: 5,
            background_color: (255, 255, 255),
            show_legend: true,
            show_grid: true,
        }
    }
}

impl PlotBuilder {
    /// Creates a new plot builder with default settings
    ///
    /// Default settings:
    /// - Resolution: 1920x1080
    /// - Font: sans-serif
    /// - Title font size: 50
    /// - Label font size: 30
    /// - Background: white
    /// - Legend and grid: enabled
    pub fn new() -> Self {
        Self::default()
    }

    // ==================== Basic Configuration ====================

    /// Sets the plot title
    pub fn title<S: AsRef<str>>(mut self, title: S) -> Self {
        log::trace!("Setting plot title to {}", title.as_ref());
        self.title = title.as_ref().to_string();
        self
    }

    /// Sets the X-axis range
    pub fn x_range(mut self, min: f32, max: f32) -> Self {
        log::trace!("Setting plot x range to {}, {}", min, max);
        self.x_range = (min, max);
        self
    }

    /// Sets the Y-axis range
    pub fn y_range(mut self, min: f32, max: f32) -> Self {
        log::trace!("Setting plot y range to {}, {}", min, max);
        self.y_range = (min, max);
        self
    }

    /// Sets the X-axis label
    pub fn x_label<S: AsRef<str>>(mut self, label: S) -> Self {
        log::trace!("Setting plot x label to {}", label.as_ref());
        self.x_label = Some(label.as_ref().to_string());
        self
    }

    /// Sets the Y-axis label
    pub fn y_label<S: AsRef<str>>(mut self, label: S) -> Self {
        log::trace!("Setting plot y label to {}", label.as_ref());
        self.y_label = Some(label.as_ref().to_string());
        self
    }

    // ==================== Data Series ====================

    /// Adds a data series to the plot
    ///
    /// # Arguments
    /// * `data` - Vector of (x, y) coordinate pairs
    /// * `label` - Name for this series (shown in legend)
    /// * `color` - Optional RGB color tuple. If None, auto-assigns a color
    ///
    /// # Example
    /// ```
    /// # use rustic::plotting::PlotBuilder;
    /// let data = vec![(0.0, 0.0), (1.0, 1.0)];
    ///
    /// PlotBuilder::new()
    ///     .add_series(data.clone(), "Series 1", Some((255, 0, 0)))  // Red
    ///     .add_series(data, "Series 2", None);  // Auto-color
    /// ```
    pub fn add_series<S: AsRef<str>>(
        mut self,
        data: Vec<(f32, f32)>,
        label: S,
        color: Option<(u8, u8, u8)>,
    ) -> Self {
        log::info!("Adding a series of {} element(s) to the plot", data.len());
        let color = color.unwrap_or_else(|| Self::auto_color(self.series.len()));
        self.series.push(SeriesConfig {
            data,
            label: label.as_ref().to_string(),
            color,
        });
        self
    }

    // ==================== Line Annotations ====================

    /// Adds a vertical line annotation at the specified x coordinate
    ///
    /// # Arguments
    /// * `x` - X coordinate for the vertical line
    /// * `color` - Optional RGB color. If None, uses gray (128, 128, 128)
    pub fn add_vertical_line(mut self, x: f32, color: Option<(u8, u8, u8)>) -> Self {
        self.lines.push(LineConfig {
            line_type: LineType::Vertical(x),
            color: color.unwrap_or((128, 128, 128)),
        });
        self
    }

    /// Adds a horizontal line annotation at the specified y coordinate
    ///
    /// # Arguments
    /// * `y` - Y coordinate for the horizontal line
    /// * `color` - Optional RGB color. If None, uses gray (128, 128, 128)
    pub fn add_horizontal_line(mut self, y: f32, color: Option<(u8, u8, u8)>) -> Self {
        self.lines.push(LineConfig {
            line_type: LineType::Horizontal(y),
            color: color.unwrap_or((128, 128, 128)),
        });
        self
    }

    /// Adds a custom line annotation between two points
    ///
    /// # Arguments
    /// * `from` - Starting point (x, y)
    /// * `to` - Ending point (x, y)
    /// * `color` - Optional RGB color. If None, uses gray (128, 128, 128)
    pub fn add_line(
        mut self,
        from: (f32, f32),
        to: (f32, f32),
        color: Option<(u8, u8, u8)>,
    ) -> Self {
        self.lines.push(LineConfig {
            line_type: LineType::Custom { from, to },
            color: color.unwrap_or((128, 128, 128)),
        });
        self
    }

    // ==================== Visual Customization ====================

    /// Sets the output image resolution in pixels
    ///
    /// # Arguments
    /// * `width` - Width in pixels
    /// * `height` - Height in pixels
    pub fn resolution(mut self, width: u32, height: u32) -> Self {
        self.resolution = (width, height);
        self
    }

    /// Sets the font family for text rendering
    ///
    /// Common options: "sans-serif", "serif", "monospace", "Arial", "Times New Roman"
    pub fn font_family<S: AsRef<str>>(mut self, family: S) -> Self {
        self.font_family = family.as_ref().to_string();
        self
    }

    /// Sets the title font size in points
    pub fn title_font_size(mut self, size: u32) -> Self {
        self.title_font_size = size;
        self
    }

    /// Sets the axis label font size in points
    pub fn label_font_size(mut self, size: u32) -> Self {
        self.label_font_size = size;
        self
    }

    /// Sets the margin around the plot in pixels
    pub fn margin(mut self, margin: u32) -> Self {
        self.margin = margin;
        self
    }

    /// Sets the background color as RGB values (0-255)
    ///
    /// # Arguments
    /// * `r` - Red component (0-255)
    /// * `g` - Green component (0-255)
    /// * `b` - Blue component (0-255)
    pub fn background_color(mut self, r: u8, g: u8, b: u8) -> Self {
        self.background_color = (r, g, b);
        self
    }

    /// Controls whether the legend is displayed
    pub fn show_legend(mut self, show: bool) -> Self {
        self.show_legend = show;
        self
    }

    /// Controls whether the grid is displayed
    pub fn show_grid(mut self, show: bool) -> Self {
        self.show_grid = show;
        self
    }

    // ==================== Terminal Method ====================

    /// Saves the plot to a file
    ///
    /// This is the terminal method that consumes the builder and generates the plot.
    ///
    /// # Arguments
    /// * `path` - Output file path (typically .png)
    ///
    /// # Errors
    /// Returns `PlotError` if:
    /// - No data series or lines have been added (`EmptyData`)
    /// - Invalid axis ranges specified (`InvalidRange`)
    /// - File cannot be written (`Io`)
    /// - Rendering fails (`Rendering`)
    pub fn save<P: AsRef<Path>>(self, path: P) -> Result<(), PlotError> {
        crate::plotting::render::render_plot(&self, path.as_ref())
    }

    // ==================== Helper Methods ====================

    /// Automatically assigns a color based on the series index
    ///
    /// Uses a palette of 10 distinct colors that cycle for additional series.
    fn auto_color(index: usize) -> (u8, u8, u8) {
        const COLORS: [(u8, u8, u8); 10] = [
            (31, 119, 180),  // Blue
            (255, 127, 14),  // Orange
            (44, 160, 44),   // Green
            (214, 39, 40),   // Red
            (148, 103, 189), // Purple
            (140, 86, 75),   // Brown
            (227, 119, 194), // Pink
            (127, 127, 127), // Gray
            (188, 189, 34),  // Yellow-green
            (23, 190, 207),  // Cyan
        ];
        COLORS[index % COLORS.len()]
    }
}
