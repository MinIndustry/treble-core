//! Error types for the plotting module

use thiserror::Error;

/// Errors that can occur during plotting operations
#[derive(Error, Debug)]
pub enum PlotError {
    /// I/O error occurred while writing the plot file
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Error during the rendering process
    #[error("Rendering error: {0}")]
    Rendering(String),

    /// Invalid data provided for plotting
    #[error("Invalid data: {0}")]
    InvalidData(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// No data series provided to plot
    #[error("Empty data series - at least one series or line must be added")]
    EmptyData,

    /// Invalid axis range specification
    #[error("Invalid range: {axis} range ({min}, {max}) is invalid (min must be less than max)")]
    InvalidRange {
        /// The axis name (X or Y)
        axis: String,
        /// Minimum value
        min: f32,
        /// Maximum value
        max: f32,
    },
}

/// Allow converting from plotters DrawingAreaErrorKind
impl<T: std::error::Error + Send + Sync> From<plotters::drawing::DrawingAreaErrorKind<T>>
    for PlotError
{
    fn from(err: plotters::drawing::DrawingAreaErrorKind<T>) -> Self {
        PlotError::Rendering(err.to_string())
    }
}
