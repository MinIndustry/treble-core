//! Core rendering logic for plots

use crate::plotting::{PlotBuilder, PlotError, types::LineType};
use plotters::prelude::*;
use std::path::Path;

/// Renders a plot based on the builder configuration
///
/// This function takes a PlotBuilder configuration and generates the actual plot image.
/// It handles validation, drawing of data series, line annotations, legends, and styling.
///
/// # Arguments
/// * `config` - The PlotBuilder containing all plot configuration
/// * `path` - Output file path
///
/// # Errors
/// Returns `PlotError` if:
/// - No data series or lines are provided (`EmptyData`)
/// - Axis ranges are invalid (`InvalidRange`)
/// - File I/O fails (`Io`)
/// - Rendering fails (`Rendering`)
pub(crate) fn render_plot(config: &PlotBuilder, path: &Path) -> Result<(), PlotError> {
    // Validation
    if config.series.is_empty() && config.lines.is_empty() {
        return Err(PlotError::EmptyData);
    }

    if config.x_range.0 >= config.x_range.1 {
        return Err(PlotError::InvalidRange {
            axis: "X".to_string(),
            min: config.x_range.0,
            max: config.x_range.1,
        });
    }

    if config.y_range.0 >= config.y_range.1 {
        return Err(PlotError::InvalidRange {
            axis: "Y".to_string(),
            min: config.y_range.0,
            max: config.y_range.1,
        });
    }

    // Setup drawing area
    let root = BitMapBackend::new(path, config.resolution).into_drawing_area();
    let bg_color = RGBColor(
        config.background_color.0,
        config.background_color.1,
        config.background_color.2,
    );
    root.fill(&bg_color)?;

    // Build chart
    let title_font = (config.font_family.as_str(), config.title_font_size).into_font();
    let label_size = config.label_font_size;

    let mut chart = ChartBuilder::on(&root)
        .caption(&config.title, title_font)
        .margin(config.margin)
        .x_label_area_size(label_size)
        .y_label_area_size(label_size)
        .build_cartesian_2d(
            config.x_range.0..config.x_range.1,
            config.y_range.0..config.y_range.1,
        )?;

    // Configure mesh (grid)
    if config.show_grid {
        let mut mesh = chart.configure_mesh();

        if let Some(ref x_label) = config.x_label {
            mesh.x_desc(x_label);
        }
        if let Some(ref y_label) = config.y_label {
            mesh.y_desc(y_label);
        }

        mesh.draw()?;
    }

    // Draw data series
    for serie_config in &config.series {
        let color = RGBColor(
            serie_config.color.0,
            serie_config.color.1,
            serie_config.color.2,
        );

        chart
            .draw_series(LineSeries::new(serie_config.data.iter().copied(), &color))?
            .label(&serie_config.label)
            .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &color));
    }

    // Draw legend
    if config.show_legend && !config.series.is_empty() {
        chart
            .configure_series_labels()
            .background_style(&WHITE.mix(0.8))
            .border_style(&BLACK)
            .draw()?;
    }

    // Draw line annotations
    for line_config in &config.lines {
        let color = RGBColor(
            line_config.color.0,
            line_config.color.1,
            line_config.color.2,
        );
        let style = ShapeStyle::from(&color).stroke_width(2);

        match line_config.line_type {
            LineType::Vertical(x) => {
                chart.draw_series(std::iter::once(PathElement::new(
                    vec![(x, config.y_range.0), (x, config.y_range.1)],
                    style,
                )))?;
            }
            LineType::Horizontal(y) => {
                chart.draw_series(std::iter::once(PathElement::new(
                    vec![(config.x_range.0, y), (config.x_range.1, y)],
                    style,
                )))?;
            }
            LineType::Custom { from, to } => {
                chart.draw_series(std::iter::once(PathElement::new(vec![from, to], style)))?;
            }
        }
    }

    log::info!("Presenting final plot");
    root.present()
        .map_err(|e| PlotError::Rendering(e.to_string()))?;

    log::info!("Plot presented");
    Ok(())
}
