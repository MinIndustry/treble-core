use plotters::style::RGBColor;

pub struct Line {
    pub from: (f32, f32),
    pub to: (f32, f32),
    pub color: RGBColor,
}
