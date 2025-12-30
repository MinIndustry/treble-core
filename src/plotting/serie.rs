//! Internal PlotSerie type (not part of public API)

use plotters::style::RGBColor;

/// Internal type for storing a data series during rendering
/// This is not part of the public API - use PlotBuilder instead
#[allow(dead_code)]
pub(crate) struct PlotSerie {
    pub name: String,
    pub data: Vec<(f32, f32)>,
    pub color: RGBColor,
}

#[allow(dead_code)]
impl PlotSerie {
    pub fn new<S: AsRef<str>>(label: S, data: Vec<(f32, f32)>, color: (u8, u8, u8)) -> Self {
        Self {
            name: label.as_ref().to_string(),
            data,
            color: RGBColor(color.0, color.1, color.2),
        }
    }
}
