use plotters::style::RGBColor;

pub struct PlotSerie {
    pub name: String,
    pub data: Vec<(f32, f32)>,
    pub color: RGBColor,
}

impl PlotSerie {
    pub fn new<S: AsRef<str>>(label: S, data: Vec<(f32, f32)>, color: (u8, u8, u8)) -> Self {
        Self {
            name: label.as_ref().to_string(),
            data,
            color: RGBColor(color.0, color.1, color.2),
        }
    }
}
