//! Internal types for the plotting module

/// Configuration for a data series
#[derive(Debug, Clone)]
pub(crate) struct SeriesConfig {
    /// The data points (x, y) to plot
    pub data: Vec<(f32, f32)>,
    /// Label for the series (shown in legend)
    pub label: String,
    /// RGB color for the series
    pub color: (u8, u8, u8),
}

/// Configuration for a line annotation
#[derive(Debug, Clone)]
pub(crate) struct LineConfig {
    /// Type of line to draw
    pub line_type: LineType,
    /// RGB color for the line
    pub color: (u8, u8, u8),
}

/// Type of line annotation
#[derive(Debug, Clone)]
pub(crate) enum LineType {
    /// Vertical line at the specified x coordinate
    Vertical(f32),
    /// Horizontal line at the specified y coordinate
    Horizontal(f32),
    /// Custom line from one point to another
    Custom {
        /// Starting point (x, y)
        from: (f32, f32),
        /// Ending point (x, y)
        to: (f32, f32),
    },
}
