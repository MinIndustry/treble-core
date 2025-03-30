use plotters::{coord::Shift, prelude::*};

mod line;
mod serie;

use line::Line;
use serie::PlotSerie;

pub struct Plot {
    pub title: String,
    pub scale: ((f32, f32), (f32, f32)),
    pub path: String,
    pub series: Vec<PlotSerie>,
    pub lines: Vec<Line>,
}

impl Plot {
    pub fn new(title: &str, x_scale: (f32, f32), y_scale: (f32, f32), filename: &str) -> Self {
        Self {
            title: title.to_string(),
            scale: (x_scale, y_scale),
            path: filename.to_string(),
            series: vec![],
            lines: vec![],
        }
    }

    pub fn plot<S: AsRef<str>>(&mut self, data: Vec<(f32, f32)>, label: S, color: (u8, u8, u8)) {
        self.series.push(PlotSerie::new(label, data, color));
    }

    pub fn render(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let root: DrawingArea<BitMapBackend<'_>, Shift> =
            BitMapBackend::new(&self.path, (1920, 1080)).into_drawing_area();
        root.fill(&WHITE)?;

        let mut chart = ChartBuilder::on(&root)
            .caption(&self.title, ("sans-serif", 50).into_font())
            .margin(5)
            .x_label_area_size(30)
            .y_label_area_size(30)
            .build_cartesian_2d(
                self.scale.0 .0..self.scale.0 .1,
                self.scale.1 .0..self.scale.1 .1,
            )?;

        chart.configure_mesh().draw()?;

        for serie in self.series.iter() {
            chart
                .draw_series(LineSeries::new(serie.data.clone(), &serie.color))?
                .label(&serie.name)
                .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &serie.color));
        }

        chart
            .configure_series_labels()
            .background_style(&WHITE.mix(0.8))
            .border_style(&BLACK)
            .draw()?;

        // for line in self.lines.iter() {
        //     root.draw(&PathElement::new(
        //         vec![line.from, line.to],
        //         Into::<ShapeStyle>::into(&line.color).filled(),
        //     ))
        // }

        root.present()?;

        Ok(())
    }
}
