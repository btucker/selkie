//! XY Chart renderer
//!
//! Renders XY charts with line and bar plots, supporting both vertical and horizontal orientations.

use crate::diagrams::xychart::{ChartOrientation, Plot, PlotType, XAxisData, XYChartDb, YAxisData};
use crate::error::Result;
use crate::render::svg::{Attrs, RenderConfig, SvgDocument, SvgElement};

/// Default chart dimensions (matching mermaid.js defaults)
const DEFAULT_WIDTH: f64 = 700.0;
const DEFAULT_HEIGHT: f64 = 500.0;

/// Layout constants matching mermaid.js config defaults
/// These values are derived from analyzing mermaid's chartBuilder
const TITLE_FONT_SIZE: f64 = 20.0;
const TITLE_PADDING: f64 = 10.0;
const AXIS_LABEL_FONT_SIZE: f64 = 14.0;
const AXIS_LABEL_PADDING: f64 = 5.0;
const AXIS_TITLE_FONT_SIZE: f64 = 16.0;
const AXIS_TITLE_PADDING: f64 = 5.0;
const TICK_LENGTH: f64 = 5.0;
const AXIS_LINE_WIDTH: f64 = 2.0;

/// Estimated character width as fraction of font size (for label width calculation)
const CHAR_WIDTH_RATIO: f64 = 0.6;
/// Estimated character height as fraction of font size
const CHAR_HEIGHT_RATIO: f64 = 1.2;

// Note: Plot colors are now taken from theme.xychart_plot_color_palette
// The palette is defined in Theme and includes the correct mermaid.js colors:
// - First color: primary_color (#ECECFF for bars)
// - Second color: #8493A6 (for lines, matching mermaid.js lineColor)

/// Chart area dimensions and data range
struct ChartArea {
    plot_left: f64,
    plot_top: f64,
    plot_width: f64,
    plot_height: f64,
    y_min: f64,
    y_max: f64,
    num_points: usize,
    /// Outer padding for bar charts (half of bar width space at edges)
    outer_padding: f64,
}

/// Layout dimensions calculated from content
struct LayoutDimensions {
    /// Width of Y-axis area (title + labels + ticks + axis line)
    y_axis_width: f64,
    /// Height of X-axis area (title + labels + ticks + axis line)
    x_axis_height: f64,
    /// Height of chart title area
    title_height: f64,
}

/// Calculate layout dimensions based on chart content (similar to mermaid's space calculation)
fn calculate_layout(db: &XYChartDb, y_min: f64, y_max: f64) -> LayoutDimensions {
    // Calculate Y-axis label width (widest number in tick values)
    let tick_values = calculate_nice_ticks(y_min, y_max);
    let max_label_len = tick_values
        .iter()
        .map(|v| format_number(*v).len())
        .max()
        .unwrap_or(1);
    let y_label_width = max_label_len as f64 * AXIS_LABEL_FONT_SIZE * CHAR_WIDTH_RATIO;

    // Y-axis width: title + padding + labels + padding + ticks + axis line
    let has_y_title = db
        .y_axis
        .as_ref()
        .map(|y| match y {
            YAxisData::Linear(d) => !d.title.is_empty(),
        })
        .unwrap_or(false);

    let y_axis_width = if has_y_title {
        AXIS_TITLE_FONT_SIZE * CHAR_HEIGHT_RATIO
            + AXIS_TITLE_PADDING * 2.0
            + y_label_width
            + AXIS_LABEL_PADDING
            + TICK_LENGTH
            + AXIS_LINE_WIDTH
    } else {
        y_label_width + AXIS_LABEL_PADDING + TICK_LENGTH + AXIS_LINE_WIDTH
    };

    // X-axis height: axis line + ticks + labels + padding + title
    let has_x_title = db
        .x_axis
        .as_ref()
        .map(|x| match x {
            XAxisData::Band(d) => !d.title.is_empty(),
            XAxisData::Linear(d) => !d.title.is_empty(),
        })
        .unwrap_or(false);

    let x_axis_height = if has_x_title {
        AXIS_LINE_WIDTH
            + TICK_LENGTH
            + AXIS_LABEL_FONT_SIZE * CHAR_HEIGHT_RATIO
            + AXIS_LABEL_PADDING * 2.0
            + AXIS_TITLE_FONT_SIZE * CHAR_HEIGHT_RATIO
            + AXIS_TITLE_PADDING
    } else {
        AXIS_LINE_WIDTH
            + TICK_LENGTH
            + AXIS_LABEL_FONT_SIZE * CHAR_HEIGHT_RATIO
            + AXIS_LABEL_PADDING * 2.0
    };

    // Title height
    let title_height = if !db.title.is_empty() {
        TITLE_FONT_SIZE * CHAR_HEIGHT_RATIO + TITLE_PADDING * 2.0
    } else {
        0.0
    };

    LayoutDimensions {
        y_axis_width,
        x_axis_height,
        title_height,
    }
}

/// Render an XY chart to SVG
pub fn render_xychart(db: &XYChartDb, config: &RenderConfig) -> Result<String> {
    let mut doc = SvgDocument::new();

    let width = DEFAULT_WIDTH;
    let height = DEFAULT_HEIGHT;
    doc.set_size(width, height);

    // Add theme styles
    if config.embed_css {
        doc.add_style(&config.theme.generate_css());
        doc.add_style(&generate_xychart_css(&config.theme));
    }

    // Calculate data range first (needed for layout calculation)
    let (y_min, y_max) = calculate_y_range(db);
    let num_points = calculate_num_points(db);

    // Calculate layout dimensions based on content
    let layout = calculate_layout(db, y_min, y_max);

    // Calculate plot area based on layout
    let plot_left = layout.y_axis_width;
    let plot_right = width; // Plot extends to right edge
    let plot_top = layout.title_height;
    let plot_bottom = height - layout.x_axis_height;

    let plot_width = plot_right - plot_left;
    let plot_height = plot_bottom - plot_top;

    // Render background
    let bg = SvgElement::Rect {
        x: 0.0,
        y: 0.0,
        width,
        height,
        rx: None,
        ry: None,
        attrs: Attrs::new()
            .with_fill(&config.theme.xychart_background_color)
            .with_class("xychart-background"),
    };
    doc.add_element(bg);

    if num_points == 0 {
        return Ok(doc.to_string());
    }

    // Calculate outer padding for bar charts
    // This matches mermaid's recalculateOuterPaddingToDrawBar logic
    let tick_distance = plot_width / num_points as f64;
    let bar_width_ratio = 0.7; // BAR_WIDTH_TO_TICK_WIDTH_RATIO in mermaid
    let outer_padding = (bar_width_ratio * tick_distance / 2.0).floor();

    let area = ChartArea {
        plot_left,
        plot_top,
        plot_width,
        plot_height,
        y_min,
        y_max,
        num_points,
        outer_padding,
    };

    // Render based on orientation
    if db.orientation == ChartOrientation::Horizontal {
        render_horizontal_chart(&mut doc, db, config, &area);
    } else {
        render_vertical_chart(&mut doc, db, config, &area);
    }

    Ok(doc.to_string())
}

/// Render a vertical chart (x-axis at bottom, y-axis at left)
/// Z-order: shapes first, then text (matching mermaid reference)
fn render_vertical_chart(
    doc: &mut SvgDocument,
    db: &XYChartDb,
    config: &RenderConfig,
    area: &ChartArea,
) {
    let plot_bottom = area.plot_top + area.plot_height;

    // PHASE 1: Render all shapes first (for correct z-order)
    // Render plot shapes (bars, lines)
    // Use theme xychart_plot_color_palette for colors (matching mermaid reference)
    let palette = &config.theme.xychart_plot_color_palette;
    let mut line_color_idx = 1; // Start at 1, as index 0 is for bars
    for plot in db.get_plots().iter() {
        match plot.plot_type {
            PlotType::Bar => {
                // Bars use first palette color (primary_color)
                let color = palette.first().map(|s| s.as_str()).unwrap_or("#ECECFF");
                render_vertical_bars(doc, plot, color, area);
            }
            PlotType::Line => {
                // Lines use subsequent palette colors starting from index 1
                let color = palette
                    .get(line_color_idx % palette.len())
                    .map(|s| s.as_str())
                    .unwrap_or("#8493A6");
                render_vertical_line(doc, plot, color, area);
                line_color_idx += 1;
            }
        }
    }

    // Render axis lines and ticks (shapes)
    render_y_axis_shapes(
        doc,
        config,
        area.plot_left,
        area.plot_top,
        area.plot_height,
        area.y_min,
        area.y_max,
        false,
    );
    render_x_axis_shapes(doc, db, config, area, plot_bottom, false);

    // PHASE 2: Render all text labels (after shapes for z-order)
    render_chart_title(doc, db, config);
    render_y_axis_text(
        doc,
        db,
        config,
        area.plot_left,
        area.plot_top,
        area.plot_height,
        area.y_min,
        area.y_max,
        false,
    );
    render_x_axis_text(doc, db, config, area, plot_bottom, false);
}

/// Render a horizontal chart (x-axis at left, y-axis at top)
/// Z-order: shapes first, then text (matching mermaid reference)
fn render_horizontal_chart(
    doc: &mut SvgDocument,
    db: &XYChartDb,
    config: &RenderConfig,
    area: &ChartArea,
) {
    // In horizontal mode, x and y are swapped visually
    // X-axis (categories) goes on the left (vertical)
    // Y-axis (values) goes on the top (horizontal)

    // PHASE 1: Render all shapes first (for correct z-order)
    // Render plot shapes (bars, lines)
    // Use theme xychart_plot_color_palette for colors (matching mermaid reference)
    let palette = &config.theme.xychart_plot_color_palette;
    let mut line_color_idx = 1; // Start at 1, as index 0 is for bars
    for plot in db.get_plots().iter() {
        match plot.plot_type {
            PlotType::Bar => {
                // Bars use first palette color (primary_color)
                let color = palette.first().map(|s| s.as_str()).unwrap_or("#ECECFF");
                render_horizontal_bars(doc, plot, color, area);
            }
            PlotType::Line => {
                // Lines use subsequent palette colors starting from index 1
                let color = palette
                    .get(line_color_idx % palette.len())
                    .map(|s| s.as_str())
                    .unwrap_or("#8493A6");
                render_horizontal_line(doc, plot, color, area);
                line_color_idx += 1;
            }
        }
    }

    // Render axis lines and ticks (shapes)
    render_y_axis_shapes(
        doc,
        config,
        area.plot_left,
        area.plot_top,
        area.plot_width,
        area.y_min,
        area.y_max,
        true,
    );
    render_x_axis_shapes(doc, db, config, area, area.plot_top, true);

    // PHASE 2: Render all text labels (after shapes for z-order)
    render_chart_title(doc, db, config);
    render_y_axis_text(
        doc,
        db,
        config,
        area.plot_left,
        area.plot_top,
        area.plot_width,
        area.y_min,
        area.y_max,
        true,
    );
    render_x_axis_text(doc, db, config, area, area.plot_top, true);
}

/// Get the x-coordinate for a data point at index i (tick-centered positioning)
/// This mimics D3's scaleBand with paddingInner(1), paddingOuter(0), align(0.5)
fn get_tick_x(area: &ChartArea, i: usize) -> f64 {
    // Calculate range with outer padding applied
    let range_start = area.plot_left + area.outer_padding;
    let range_end = area.plot_left + area.plot_width - area.outer_padding;
    let range_width = range_end - range_start;

    if area.num_points <= 1 {
        // Single point centered
        range_start + range_width / 2.0
    } else {
        // Points distributed evenly across the range
        range_start + range_width * (i as f64) / (area.num_points - 1) as f64
    }
}

/// Render vertical bars for a bar plot
fn render_vertical_bars(doc: &mut SvgDocument, plot: &Plot, color: &str, area: &ChartArea) {
    // Bar width calculation matching mermaid: min(outerPadding * 2, tickDistance) * (1 - barPaddingPercent)
    let tick_distance = if area.num_points > 1 {
        let range_width = area.plot_width - 2.0 * area.outer_padding;
        range_width / (area.num_points - 1) as f64
    } else {
        area.plot_width
    };

    let bar_padding_percent = 0.05;
    let bar_width = (area.outer_padding * 2.0).min(tick_distance) * (1.0 - bar_padding_percent);
    let bar_width_half = bar_width / 2.0;

    let plot_bottom = area.plot_top + area.plot_height;
    let y_range = area.y_max - area.y_min;

    for (i, data_point) in plot.data.iter().enumerate() {
        let tick_x = get_tick_x(area, i);
        let x = tick_x - bar_width_half;

        // Calculate bar height and y position
        let value_ratio = if y_range != 0.0 {
            (data_point.value - area.y_min) / y_range
        } else {
            0.5
        };

        let bar_height = area.plot_height * value_ratio;
        let y = plot_bottom - bar_height;

        // Handle negative values
        let (actual_y, actual_height) = if data_point.value >= 0.0 {
            (y, bar_height)
        } else {
            // For negative values, the bar goes down from the zero line
            let zero_y = if y_range != 0.0 {
                plot_bottom - area.plot_height * ((0.0 - area.y_min) / y_range)
            } else {
                plot_bottom
            };
            (zero_y, bar_height.abs())
        };

        let bar = SvgElement::Rect {
            x,
            y: actual_y,
            width: bar_width.max(1.0),
            height: actual_height.max(0.0),
            rx: None,
            ry: None,
            attrs: Attrs::new().with_fill(color).with_class("xychart-bar"),
        };
        doc.add_element(bar);
    }
}

/// Get the y-coordinate for a data point at index i in horizontal mode (tick-centered)
fn get_tick_y(area: &ChartArea, i: usize) -> f64 {
    // Calculate range with outer padding applied
    let range_start = area.plot_top + area.outer_padding;
    let range_end = area.plot_top + area.plot_height - area.outer_padding;
    let range_height = range_end - range_start;

    if area.num_points <= 1 {
        range_start + range_height / 2.0
    } else {
        range_start + range_height * (i as f64) / (area.num_points - 1) as f64
    }
}

/// Render horizontal bars for a bar plot
fn render_horizontal_bars(doc: &mut SvgDocument, plot: &Plot, color: &str, area: &ChartArea) {
    // Bar height calculation for horizontal mode
    let tick_distance = if area.num_points > 1 {
        let range_height = area.plot_height - 2.0 * area.outer_padding;
        range_height / (area.num_points - 1) as f64
    } else {
        area.plot_height
    };

    let bar_padding_percent = 0.05;
    let bar_height = (area.outer_padding * 2.0).min(tick_distance) * (1.0 - bar_padding_percent);
    let bar_height_half = bar_height / 2.0;

    let y_range = area.y_max - area.y_min;

    for (i, data_point) in plot.data.iter().enumerate() {
        let tick_y = get_tick_y(area, i);
        let y = tick_y - bar_height_half;

        // Calculate bar width
        let value_ratio = if y_range != 0.0 {
            (data_point.value - area.y_min) / y_range
        } else {
            0.5
        };

        let bar_width_calc = area.plot_width * value_ratio;

        let bar = SvgElement::Rect {
            x: area.plot_left,
            y,
            width: bar_width_calc.max(0.0),
            height: bar_height.max(1.0),
            rx: None,
            ry: None,
            attrs: Attrs::new().with_fill(color).with_class("xychart-bar"),
        };
        doc.add_element(bar);
    }
}

/// Render a line for a line plot (vertical orientation)
/// Note: Unlike some implementations, mermaid does NOT draw circles at data points
fn render_vertical_line(doc: &mut SvgDocument, plot: &Plot, color: &str, area: &ChartArea) {
    if plot.data.is_empty() {
        return;
    }

    let plot_bottom = area.plot_top + area.plot_height;
    let y_range = area.y_max - area.y_min;

    let mut path_data = String::new();

    for (i, data_point) in plot.data.iter().enumerate() {
        // Use same tick positioning as bars
        let x = get_tick_x(area, i);

        let value_ratio = if y_range != 0.0 {
            (data_point.value - area.y_min) / y_range
        } else {
            0.5
        };
        let y = plot_bottom - area.plot_height * value_ratio;

        if i == 0 {
            path_data.push_str(&format!("M {} {}", x, y));
        } else {
            path_data.push_str(&format!(" L {} {}", x, y));
        }
    }

    let line = SvgElement::Path {
        d: path_data,
        attrs: Attrs::new()
            .with_fill("none")
            .with_stroke(color)
            .with_stroke_width(2.0)
            .with_class("xychart-line"),
    };
    doc.add_element(line);
    // Note: mermaid reference does not render circles at data points
}

/// Render a line for a line plot (horizontal orientation)
/// Note: Unlike some implementations, mermaid does NOT draw circles at data points
fn render_horizontal_line(doc: &mut SvgDocument, plot: &Plot, color: &str, area: &ChartArea) {
    if plot.data.is_empty() {
        return;
    }

    let y_range = area.y_max - area.y_min;

    let mut path_data = String::new();

    for (i, data_point) in plot.data.iter().enumerate() {
        // Use same tick positioning as horizontal bars
        let y = get_tick_y(area, i);

        let value_ratio = if y_range != 0.0 {
            (data_point.value - area.y_min) / y_range
        } else {
            0.5
        };
        let x = area.plot_left + area.plot_width * value_ratio;

        if i == 0 {
            path_data.push_str(&format!("M {} {}", x, y));
        } else {
            path_data.push_str(&format!(" L {} {}", x, y));
        }
    }

    let line = SvgElement::Path {
        d: path_data,
        attrs: Attrs::new()
            .with_fill("none")
            .with_stroke(color)
            .with_stroke_width(2.0)
            .with_class("xychart-line"),
    };
    doc.add_element(line);
    // Note: mermaid reference does not render circles at data points
}

/// Render chart title - called after shapes for proper z-order
fn render_chart_title(doc: &mut SvgDocument, db: &XYChartDb, config: &RenderConfig) {
    if !db.title.is_empty() {
        // Position title centered horizontally, with padding from top
        // Mermaid uses titlePadding + titleHeight/2 with dominant-baseline: middle
        let title_y = TITLE_PADDING + (TITLE_FONT_SIZE * CHAR_HEIGHT_RATIO) / 2.0;
        let title_elem = SvgElement::Text {
            x: DEFAULT_WIDTH / 2.0,
            y: title_y,
            content: db.title.clone(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_attr("dominant-baseline", "middle")
                .with_class("xychart-title")
                .with_attr("font-size", "20")
                .with_attr("font-weight", "bold")
                .with_fill(&config.theme.xychart_title_color),
        };
        doc.add_element(title_elem);
    }
}

/// Render Y-axis shapes (axis line and tick marks) - for z-order separation
#[allow(clippy::too_many_arguments)]
fn render_y_axis_shapes(
    doc: &mut SvgDocument,
    config: &RenderConfig,
    plot_left: f64,
    plot_top: f64,
    axis_length: f64,
    y_min: f64,
    y_max: f64,
    is_horizontal: bool,
) {
    // Axis line
    let (x1, y1, x2, y2) = if is_horizontal {
        (plot_left, plot_top, plot_left + axis_length, plot_top)
    } else {
        (plot_left, plot_top, plot_left, plot_top + axis_length)
    };

    let axis_line = SvgElement::Path {
        d: format!("M {} {} L {} {}", x1, y1, x2, y2),
        attrs: Attrs::new()
            .with_stroke(&config.theme.xychart_y_axis_line_color)
            .with_stroke_width(2.0)
            .with_fill("none")
            .with_class("xychart-axis"),
    };
    doc.add_element(axis_line);

    // Y-axis tick marks
    let tick_values = calculate_nice_ticks(y_min, y_max);

    for value in &tick_values {
        let ratio = if (y_max - y_min).abs() > f64::EPSILON {
            (value - y_min) / (y_max - y_min)
        } else {
            0.5
        };

        let (tick_x, tick_y) = if is_horizontal {
            let x = plot_left + axis_length * ratio;
            (x, plot_top)
        } else {
            let y = plot_top + axis_length * (1.0 - ratio);
            (plot_left, y)
        };

        let (tick_dx, tick_dy) = if is_horizontal {
            (0.0, -TICK_LENGTH)
        } else {
            (-TICK_LENGTH, 0.0)
        };

        let tick = SvgElement::Path {
            d: format!(
                "M {} {} L {} {}",
                tick_x,
                tick_y,
                tick_x + tick_dx,
                tick_y + tick_dy
            ),
            attrs: Attrs::new()
                .with_stroke(&config.theme.xychart_y_axis_tick_color)
                .with_stroke_width(2.0)
                .with_fill("none")
                .with_class("xychart-tick"),
        };
        doc.add_element(tick);
    }
}

/// Render Y-axis text (title and labels) - for z-order separation
#[allow(clippy::too_many_arguments)]
fn render_y_axis_text(
    doc: &mut SvgDocument,
    db: &XYChartDb,
    config: &RenderConfig,
    plot_left: f64,
    plot_top: f64,
    axis_length: f64,
    y_min: f64,
    y_max: f64,
    is_horizontal: bool,
) {
    // Y-axis title
    if let Some(YAxisData::Linear(axis_data)) = &db.y_axis {
        if !axis_data.title.is_empty() {
            let (title_x, title_y, rotation) = if is_horizontal {
                (plot_left + axis_length / 2.0, plot_top - 25.0, 0.0)
            } else {
                (15.0, plot_top + axis_length / 2.0, -90.0)
            };

            let title = SvgElement::Text {
                x: title_x,
                y: title_y,
                content: axis_data.title.clone(),
                attrs: Attrs::new()
                    .with_attr("text-anchor", "middle")
                    .with_attr("dominant-baseline", "middle")
                    .with_attr(
                        "transform",
                        &format!("rotate({} {} {})", rotation, title_x, title_y),
                    )
                    .with_class("xychart-axis-title")
                    .with_attr("font-size", "16")
                    .with_fill(&config.theme.xychart_y_axis_title_color),
            };
            doc.add_element(title);
        }
    }

    // Y-axis labels
    let tick_values = calculate_nice_ticks(y_min, y_max);

    for value in &tick_values {
        let ratio = if (y_max - y_min).abs() > f64::EPSILON {
            (value - y_min) / (y_max - y_min)
        } else {
            0.5
        };

        let (label_x, label_y) = if is_horizontal {
            let x = plot_left + axis_length * ratio;
            (x, plot_top - 10.0)
        } else {
            let y = plot_top + axis_length * (1.0 - ratio);
            (plot_left - 10.0, y)
        };

        let label_text = format_number(*value);
        let label = SvgElement::Text {
            x: label_x,
            y: label_y,
            content: label_text,
            attrs: Attrs::new()
                .with_attr("text-anchor", if is_horizontal { "middle" } else { "end" })
                .with_attr(
                    "dominant-baseline",
                    if is_horizontal { "auto" } else { "middle" },
                )
                .with_class("xychart-axis-label")
                .with_attr("font-size", "14")
                .with_fill(&config.theme.xychart_y_axis_label_color),
        };
        doc.add_element(label);
    }
}

/// Render X-axis shapes (axis line and tick marks) - for z-order separation
#[allow(clippy::too_many_arguments)]
fn render_x_axis_shapes(
    doc: &mut SvgDocument,
    db: &XYChartDb,
    config: &RenderConfig,
    area: &ChartArea,
    axis_y: f64,
    is_horizontal: bool,
) {
    let plot_left = area.plot_left;
    let axis_length = if is_horizontal {
        area.plot_height
    } else {
        area.plot_width
    };

    // Axis line
    let (x1, y1, x2, y2) = if is_horizontal {
        (plot_left, axis_y, plot_left, axis_y + axis_length)
    } else {
        (plot_left, axis_y, plot_left + axis_length, axis_y)
    };

    let axis_line = SvgElement::Path {
        d: format!("M {} {} L {} {}", x1, y1, x2, y2),
        attrs: Attrs::new()
            .with_stroke(&config.theme.xychart_x_axis_line_color)
            .with_stroke_width(2.0)
            .with_fill("none")
            .with_class("xychart-axis"),
    };
    doc.add_element(axis_line);

    // Get category labels for tick count
    let categories = get_x_axis_categories(db, area.num_points);

    // X-axis tick marks at tick-centered positions
    for i in 0..categories.len() {
        let (tick_x, tick_y) = if is_horizontal {
            let y = get_tick_y(area, i);
            (plot_left, y)
        } else {
            let x = get_tick_x(area, i);
            (x, axis_y)
        };

        let (tick_dx, tick_dy) = if is_horizontal {
            (-TICK_LENGTH, 0.0)
        } else {
            (0.0, TICK_LENGTH)
        };

        let tick = SvgElement::Path {
            d: format!(
                "M {} {} L {} {}",
                tick_x,
                tick_y,
                tick_x + tick_dx,
                tick_y + tick_dy
            ),
            attrs: Attrs::new()
                .with_stroke(&config.theme.xychart_x_axis_tick_color)
                .with_stroke_width(2.0)
                .with_fill("none")
                .with_class("xychart-tick"),
        };
        doc.add_element(tick);
    }
}

/// Render X-axis text (title and labels) - for z-order separation
#[allow(clippy::too_many_arguments)]
fn render_x_axis_text(
    doc: &mut SvgDocument,
    db: &XYChartDb,
    config: &RenderConfig,
    area: &ChartArea,
    axis_y: f64,
    is_horizontal: bool,
) {
    let plot_left = area.plot_left;
    let axis_length = if is_horizontal {
        area.plot_height
    } else {
        area.plot_width
    };

    // Get category labels
    let categories = get_x_axis_categories(db, area.num_points);

    // X-axis title
    if let Some(x_axis) = &db.x_axis {
        let title = match x_axis {
            XAxisData::Band(axis_data) => &axis_data.title,
            XAxisData::Linear(axis_data) => &axis_data.title,
        };

        if !title.is_empty() {
            let (title_x, title_y, rotation) = if is_horizontal {
                (plot_left - 25.0, axis_y + axis_length / 2.0, -90.0)
            } else {
                (plot_left + axis_length / 2.0, axis_y + 35.0, 0.0)
            };

            let title_elem = SvgElement::Text {
                x: title_x,
                y: title_y,
                content: title.clone(),
                attrs: Attrs::new()
                    .with_attr("text-anchor", "middle")
                    .with_attr("dominant-baseline", "middle")
                    .with_attr(
                        "transform",
                        &format!("rotate({} {} {})", rotation, title_x, title_y),
                    )
                    .with_class("xychart-axis-title")
                    .with_attr("font-size", "16")
                    .with_fill(&config.theme.xychart_x_axis_title_color),
            };
            doc.add_element(title_elem);
        }
    }

    // X-axis labels at tick-centered positions
    // Position: axisY + labelPadding + tickLength + axisLineWidth (matching mermaid)
    let label_offset = AXIS_LABEL_PADDING + TICK_LENGTH + AXIS_LINE_WIDTH;
    for (i, category) in categories.iter().enumerate() {
        let (label_x, label_y) = if is_horizontal {
            let y = get_tick_y(area, i);
            (plot_left - 10.0, y)
        } else {
            let x = get_tick_x(area, i);
            (x, axis_y + label_offset)
        };

        let label = SvgElement::Text {
            x: label_x,
            y: label_y,
            content: truncate_label(category, 10),
            attrs: Attrs::new()
                .with_attr("text-anchor", if is_horizontal { "end" } else { "middle" })
                .with_attr(
                    "dominant-baseline",
                    if is_horizontal {
                        "middle"
                    } else {
                        "text-before-edge"
                    },
                )
                .with_class("xychart-axis-label")
                .with_attr("font-size", "14")
                .with_fill(&config.theme.xychart_x_axis_label_color),
        };
        doc.add_element(label);
    }
}

/// Calculate the Y-axis range from the data
fn calculate_y_range(db: &XYChartDb) -> (f64, f64) {
    // First check if Y-axis has explicit range
    if let Some(YAxisData::Linear(axis_data)) = &db.y_axis {
        if axis_data.min != 0.0 || axis_data.max != 0.0 {
            return (axis_data.min, axis_data.max);
        }
    }

    // Otherwise calculate from data
    let mut min = f64::MAX;
    let mut max = f64::MIN;

    for plot in db.get_plots() {
        for data_point in &plot.data {
            min = min.min(data_point.value);
            max = max.max(data_point.value);
        }
    }

    if min == f64::MAX || max == f64::MIN {
        return (0.0, 100.0); // Default range
    }

    // Add some padding
    let range = max - min;
    let padding = if range > 0.0 { range * 0.1 } else { 1.0 };

    // Include 0 if data is all positive or all negative
    if min >= 0.0 {
        min = 0.0;
    }

    (min - padding.min(min.abs() * 0.1), max + padding)
}

/// Calculate the number of data points
fn calculate_num_points(db: &XYChartDb) -> usize {
    // First check x-axis for categories
    if let Some(XAxisData::Band(axis_data)) = &db.x_axis {
        if !axis_data.categories.is_empty() {
            return axis_data.categories.len();
        }
    }

    // Otherwise get max from plots
    db.get_plots()
        .iter()
        .map(|p| p.data.len())
        .max()
        .unwrap_or(0)
}

/// Get X-axis category labels
fn get_x_axis_categories(db: &XYChartDb, num_points: usize) -> Vec<String> {
    if let Some(XAxisData::Band(axis_data)) = &db.x_axis {
        if !axis_data.categories.is_empty() {
            return axis_data.categories.clone();
        }
    }

    // Generate numeric labels
    (1..=num_points).map(|i| i.to_string()).collect()
}

/// Format a number for display
fn format_number(value: f64) -> String {
    if value.fract() == 0.0 || value.abs() >= 1000.0 {
        format!("{:.0}", value)
    } else {
        format!("{:.1}", value)
    }
}

/// Calculate "nice" tick values similar to D3's scale.ticks()
/// This generates evenly spaced, human-friendly tick values
fn calculate_nice_ticks(min: f64, max: f64) -> Vec<f64> {
    let range = max - min;
    if range <= 0.0 {
        return vec![min];
    }

    // Target approximately 10 ticks (like D3's default)
    let target_ticks = 10;

    // Calculate the rough step size
    let rough_step = range / target_ticks as f64;

    // Find a "nice" step size (1, 2, 5, 10, 20, 50, etc.)
    let magnitude = 10.0_f64.powf(rough_step.log10().floor());
    let residual = rough_step / magnitude;

    let nice_step = if residual <= 1.0 {
        magnitude
    } else if residual <= 2.0 {
        2.0 * magnitude
    } else if residual <= 5.0 {
        5.0 * magnitude
    } else {
        10.0 * magnitude
    };

    // Generate ticks starting from a "nice" value
    let nice_min = (min / nice_step).floor() * nice_step;
    let nice_max = (max / nice_step).ceil() * nice_step;

    let mut ticks = Vec::new();
    let mut tick = nice_min;

    while tick <= nice_max + f64::EPSILON {
        // Only include ticks within the actual data range
        if tick >= min - f64::EPSILON && tick <= max + f64::EPSILON {
            ticks.push(tick);
        }
        tick += nice_step;
    }

    // Ensure we have at least min and max
    if ticks.is_empty() {
        ticks.push(min);
        if (max - min).abs() > f64::EPSILON {
            ticks.push(max);
        }
    }

    ticks
}

/// Truncate a label if too long
fn truncate_label(label: &str, max_len: usize) -> String {
    if label.len() > max_len {
        format!("{}...", &label[..max_len - 3])
    } else {
        label.to_string()
    }
}

/// Generate CSS for XY chart styling
fn generate_xychart_css(theme: &crate::render::svg::Theme) -> String {
    format!(
        r#"
.xychart-background {{
  fill: {background};
}}

.xychart-title {{
  fill: {title_color};
  font-family: {font_family};
}}

.xychart-axis {{
  stroke: {axis_line_color};
  stroke-width: 2px;
}}

.xychart-axis-title {{
  fill: {axis_title_color};
  font-family: {font_family};
}}

.xychart-axis-label {{
  fill: {axis_label_color};
  font-family: {font_family};
}}

.xychart-tick {{
  stroke: {tick_color};
  stroke-width: 2px;
}}

.xychart-bar {{
  stroke-width: 0;
}}

.xychart-line {{
  fill: none;
  stroke-width: 2px;
}}

.xychart-point {{
  stroke-width: 1px;
}}
"#,
        background = theme.xychart_background_color,
        title_color = theme.xychart_title_color,
        axis_line_color = theme.xychart_x_axis_line_color,
        axis_title_color = theme.xychart_x_axis_title_color,
        axis_label_color = theme.xychart_x_axis_label_color,
        tick_color = theme.xychart_x_axis_tick_color,
        font_family = theme.font_family,
    )
}
