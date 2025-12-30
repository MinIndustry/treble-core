//! Internal Line type (not part of public API)

use plotters::style::RGBColor;

/// Internal type for line annotations
/// This is not part of the public API - use PlotBuilder's line methods instead
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct Line {
    pub from: (f32, f32),
    pub to: (f32, f32),
    pub color: RGBColor,
}

#[allow(dead_code)]
impl Line {
    pub fn new(from: (f32, f32), to: (f32, f32), color: (u8, u8, u8)) -> Self {
        Self {
            from,
            to,
            color: RGBColor(color.0, color.1, color.2),
        }
    }
}
