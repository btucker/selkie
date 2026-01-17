//! SVG element types

use std::collections::HashMap;
use std::fmt::Write;

use crate::layout::Point;

/// SVG attributes
#[derive(Debug, Clone, Default)]
pub struct Attrs {
    attrs: HashMap<String, String>,
    classes: Vec<String>,
}

impl Attrs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_class(mut self, class: &str) -> Self {
        self.classes.push(class.to_string());
        self
    }

    pub fn with_id(mut self, id: &str) -> Self {
        self.attrs.insert("id".to_string(), id.to_string());
        self
    }

    pub fn with_attr(mut self, key: &str, value: &str) -> Self {
        self.attrs.insert(key.to_string(), value.to_string());
        self
    }

    pub fn with_style(mut self, style: &str) -> Self {
        self.attrs.insert("style".to_string(), style.to_string());
        self
    }

    pub fn with_transform(mut self, transform: &str) -> Self {
        self.attrs
            .insert("transform".to_string(), transform.to_string());
        self
    }

    pub fn with_fill(mut self, fill: &str) -> Self {
        self.attrs.insert("fill".to_string(), fill.to_string());
        self
    }

    pub fn with_stroke(mut self, stroke: &str) -> Self {
        self.attrs.insert("stroke".to_string(), stroke.to_string());
        self
    }

    pub fn with_stroke_width(mut self, width: f64) -> Self {
        self.attrs
            .insert("stroke-width".to_string(), format!("{}", width));
        self
    }

    pub fn with_stroke_dasharray(mut self, dasharray: &str) -> Self {
        self.attrs
            .insert("stroke-dasharray".to_string(), dasharray.to_string());
        self
    }

    /// Convert to SVG attribute string
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        let mut result = String::new();

        if !self.classes.is_empty() {
            write!(result, " class=\"{}\"", self.classes.join(" ")).unwrap();
        }

        for (key, value) in &self.attrs {
            write!(result, " {}=\"{}\"", key, escape_xml(value)).unwrap();
        }

        result
    }
}

/// Escape special XML characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// SVG element types
#[derive(Debug, Clone)]
pub enum SvgElement {
    /// Rectangle element
    Rect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        rx: Option<f64>,
        ry: Option<f64>,
        attrs: Attrs,
    },
    /// Circle element
    Circle {
        cx: f64,
        cy: f64,
        r: f64,
        attrs: Attrs,
    },
    /// Ellipse element
    Ellipse {
        cx: f64,
        cy: f64,
        rx: f64,
        ry: f64,
        attrs: Attrs,
    },
    /// Polygon element with Point vector
    Polygon { points: Vec<Point>, attrs: Attrs },
    /// Polygon element with points as string (for complex shapes)
    PolygonStr { points: String, attrs: Attrs },
    /// Path element
    Path { d: String, attrs: Attrs },
    /// Line element
    Line {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        attrs: Attrs,
    },
    /// Polyline element
    Polyline { points: Vec<Point>, attrs: Attrs },
    /// Text element
    Text {
        x: f64,
        y: f64,
        content: String,
        attrs: Attrs,
    },
    /// Group element
    Group {
        children: Vec<SvgElement>,
        attrs: Attrs,
    },
    /// Definitions element
    Defs { children: Vec<SvgElement> },
    /// Marker element
    Marker {
        id: String,
        view_box: String,
        ref_x: f64,
        ref_y: f64,
        marker_width: f64,
        marker_height: f64,
        orient: String,
        marker_units: Option<String>,
        children: Vec<SvgElement>,
    },
    /// Style element (for embedded CSS)
    Style { content: String },
    /// Raw SVG content
    Raw { content: String },
}

impl SvgElement {
    /// Create a rectangle
    pub fn rect(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self::Rect {
            x,
            y,
            width,
            height,
            rx: None,
            ry: None,
            attrs: Attrs::new(),
        }
    }

    /// Create a rounded rectangle
    pub fn rounded_rect(x: f64, y: f64, width: f64, height: f64, rx: f64) -> Self {
        Self::Rect {
            x,
            y,
            width,
            height,
            rx: Some(rx),
            ry: Some(rx),
            attrs: Attrs::new(),
        }
    }

    /// Create a circle
    pub fn circle(cx: f64, cy: f64, r: f64) -> Self {
        Self::Circle {
            cx,
            cy,
            r,
            attrs: Attrs::new(),
        }
    }

    /// Create a polygon from points
    pub fn polygon(points: Vec<Point>) -> Self {
        Self::Polygon {
            points,
            attrs: Attrs::new(),
        }
    }

    /// Create a path
    pub fn path(d: impl Into<String>) -> Self {
        Self::Path {
            d: d.into(),
            attrs: Attrs::new(),
        }
    }

    /// Create a polyline
    pub fn polyline(points: Vec<Point>) -> Self {
        Self::Polyline {
            points,
            attrs: Attrs::new(),
        }
    }

    /// Create a text element
    pub fn text(x: f64, y: f64, content: impl Into<String>) -> Self {
        Self::Text {
            x,
            y,
            content: content.into(),
            attrs: Attrs::new(),
        }
    }

    /// Create a group
    pub fn group(children: Vec<SvgElement>) -> Self {
        Self::Group {
            children,
            attrs: Attrs::new(),
        }
    }

    /// Add inline style attribute
    pub fn with_style(self, style: &str) -> Self {
        match self {
            Self::Rect {
                x,
                y,
                width,
                height,
                rx,
                ry,
                attrs,
            } => Self::Rect {
                x,
                y,
                width,
                height,
                rx,
                ry,
                attrs: attrs.with_style(style),
            },
            Self::Circle { cx, cy, r, attrs } => Self::Circle {
                cx,
                cy,
                r,
                attrs: attrs.with_style(style),
            },
            Self::Ellipse {
                cx,
                cy,
                rx,
                ry,
                attrs,
            } => Self::Ellipse {
                cx,
                cy,
                rx,
                ry,
                attrs: attrs.with_style(style),
            },
            Self::Polygon { points, attrs } => Self::Polygon {
                points,
                attrs: attrs.with_style(style),
            },
            Self::PolygonStr { points, attrs } => Self::PolygonStr {
                points,
                attrs: attrs.with_style(style),
            },
            Self::Path { d, attrs } => Self::Path {
                d,
                attrs: attrs.with_style(style),
            },
            Self::Line {
                x1,
                y1,
                x2,
                y2,
                attrs,
            } => Self::Line {
                x1,
                y1,
                x2,
                y2,
                attrs: attrs.with_style(style),
            },
            Self::Polyline { points, attrs } => Self::Polyline {
                points,
                attrs: attrs.with_style(style),
            },
            Self::Text {
                x,
                y,
                content,
                attrs,
            } => Self::Text {
                x,
                y,
                content,
                attrs: attrs.with_style(style),
            },
            Self::Group { children, attrs } => Self::Group {
                children,
                attrs: attrs.with_style(style),
            },
            other => other,
        }
    }

    /// Add attributes
    pub fn with_attrs(self, attrs: Attrs) -> Self {
        match self {
            Self::Rect {
                x,
                y,
                width,
                height,
                rx,
                ry,
                ..
            } => Self::Rect {
                x,
                y,
                width,
                height,
                rx,
                ry,
                attrs,
            },
            Self::Circle { cx, cy, r, .. } => Self::Circle { cx, cy, r, attrs },
            Self::Ellipse { cx, cy, rx, ry, .. } => Self::Ellipse {
                cx,
                cy,
                rx,
                ry,
                attrs,
            },
            Self::Polygon { points, .. } => Self::Polygon { points, attrs },
            Self::PolygonStr { points, .. } => Self::PolygonStr { points, attrs },
            Self::Path { d, .. } => Self::Path { d, attrs },
            Self::Line { x1, y1, x2, y2, .. } => Self::Line {
                x1,
                y1,
                x2,
                y2,
                attrs,
            },
            Self::Polyline { points, .. } => Self::Polyline { points, attrs },
            Self::Text { x, y, content, .. } => Self::Text {
                x,
                y,
                content,
                attrs,
            },
            Self::Group { children, .. } => Self::Group { children, attrs },
            other => other,
        }
    }

    /// Render to SVG string
    pub fn to_svg(&self, indent: usize) -> String {
        let indent_str = "  ".repeat(indent);

        match self {
            Self::Rect {
                x,
                y,
                width,
                height,
                rx,
                ry,
                attrs,
            } => {
                let rx_str = rx.map(|v| format!(" rx=\"{}\"", v)).unwrap_or_default();
                let ry_str = ry.map(|v| format!(" ry=\"{}\"", v)).unwrap_or_default();
                format!(
                    "{}<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{}{}{}/>",
                    indent_str,
                    x,
                    y,
                    width,
                    height,
                    rx_str,
                    ry_str,
                    attrs.to_string()
                )
            }
            Self::Circle { cx, cy, r, attrs } => {
                format!(
                    "{}<circle cx=\"{}\" cy=\"{}\" r=\"{}\"{}/>",
                    indent_str,
                    cx,
                    cy,
                    r,
                    attrs.to_string()
                )
            }
            Self::Ellipse {
                cx,
                cy,
                rx,
                ry,
                attrs,
            } => {
                format!(
                    "{}<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\"{}/>",
                    indent_str,
                    cx,
                    cy,
                    rx,
                    ry,
                    attrs.to_string()
                )
            }
            Self::Polygon { points, attrs } => {
                let points_str: String = points
                    .iter()
                    .map(|p| format!("{},{}", p.x, p.y))
                    .collect::<Vec<_>>()
                    .join(" ");
                format!(
                    "{}<polygon points=\"{}\"{}/>",
                    indent_str,
                    points_str,
                    attrs.to_string()
                )
            }
            Self::PolygonStr { points, attrs } => {
                format!(
                    "{}<polygon points=\"{}\"{}/>",
                    indent_str,
                    points,
                    attrs.to_string()
                )
            }
            Self::Path { d, attrs } => {
                format!("{}<path d=\"{}\"{}/>", indent_str, d, attrs.to_string())
            }
            Self::Line {
                x1,
                y1,
                x2,
                y2,
                attrs,
            } => {
                format!(
                    "{}<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"{}/>",
                    indent_str,
                    x1,
                    y1,
                    x2,
                    y2,
                    attrs.to_string()
                )
            }
            Self::Polyline { points, attrs } => {
                let points_str: String = points
                    .iter()
                    .map(|p| format!("{},{}", p.x, p.y))
                    .collect::<Vec<_>>()
                    .join(" ");
                format!(
                    "{}<polyline points=\"{}\"{}/>",
                    indent_str,
                    points_str,
                    attrs.to_string()
                )
            }
            Self::Text {
                x,
                y,
                content,
                attrs,
            } => {
                let normalized = content
                    .replace("<br />", "\n")
                    .replace("<br/>", "\n")
                    .replace("<br>", "\n");
                if normalized.contains('\n') {
                    let lines: Vec<&str> = normalized.split('\n').collect();
                    let num_lines = lines.len();
                    // Calculate vertical offset to center the text block
                    // For n lines with 1.2em spacing, offset first line by -(n-1)*0.6em
                    let first_line_offset = if num_lines > 1 {
                        -((num_lines - 1) as f64 * 0.6)
                    } else {
                        0.0
                    };
                    let tspans = lines
                        .iter()
                        .enumerate()
                        .map(|(idx, line)| {
                            if idx == 0 {
                                if num_lines > 1 {
                                    // Use dy to offset from the y position for centering
                                    format!(
                                        "<tspan x=\"{}\" y=\"{}\" dy=\"{}em\">{}</tspan>",
                                        x,
                                        y,
                                        first_line_offset,
                                        escape_xml(line)
                                    )
                                } else {
                                    format!(
                                        "<tspan x=\"{}\" y=\"{}\">{}</tspan>",
                                        x,
                                        y,
                                        escape_xml(line)
                                    )
                                }
                            } else {
                                format!(
                                    "<tspan x=\"{}\" dy=\"1.2em\">{}</tspan>",
                                    x,
                                    escape_xml(line)
                                )
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("");
                    format!(
                        "{}<text x=\"{}\" y=\"{}\"{}>{}</text>",
                        indent_str,
                        x,
                        y,
                        attrs.to_string(),
                        tspans
                    )
                } else {
                    format!(
                        "{}<text x=\"{}\" y=\"{}\"{}>{}</text>",
                        indent_str,
                        x,
                        y,
                        attrs.to_string(),
                        escape_xml(&normalized)
                    )
                }
            }
            Self::Group { children, attrs } => {
                let children_str: String = children
                    .iter()
                    .map(|c| c.to_svg(indent + 1))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!(
                    "{}<g{}>\n{}\n{}</g>",
                    indent_str,
                    attrs.to_string(),
                    children_str,
                    indent_str
                )
            }
            Self::Defs { children } => {
                let children_str: String = children
                    .iter()
                    .map(|c| c.to_svg(indent + 1))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!(
                    "{}<defs>\n{}\n{}</defs>",
                    indent_str, children_str, indent_str
                )
            }
            Self::Marker {
                id,
                view_box,
                ref_x,
                ref_y,
                marker_width,
                marker_height,
                orient,
                marker_units,
                children,
            } => {
                let children_str: String = children
                    .iter()
                    .map(|c| c.to_svg(indent + 1))
                    .collect::<Vec<_>>()
                    .join("\n");
                let marker_units_str = marker_units
                    .as_ref()
                    .map(|u| format!(" markerUnits=\"{}\"", u))
                    .unwrap_or_default();
                let view_box_str = if view_box.is_empty() {
                    String::new()
                } else {
                    format!(" viewBox=\"{}\"", view_box)
                };
                format!(
                    "{}<marker id=\"{}\"{} refX=\"{}\" refY=\"{}\" markerWidth=\"{}\" markerHeight=\"{}\" orient=\"{}\"{}>\n{}\n{}</marker>",
                    indent_str, id, view_box_str, ref_x, ref_y, marker_width, marker_height, orient, marker_units_str, children_str, indent_str
                )
            }
            Self::Style { content } => {
                format!("{}<style>\n{}\n{}</style>", indent_str, content, indent_str)
            }
            Self::Raw { content } => {
                format!("{}{}", indent_str, content)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Attrs, SvgElement};

    #[test]
    fn text_br_splits_into_tspans() {
        let element = SvgElement::Text {
            x: 10.0,
            y: 20.0,
            content: "Line 1<br/>Line 2".to_string(),
            attrs: Attrs::new(),
        };
        let svg = element.to_svg(0);

        // For 2 lines, first line should be offset by -0.6em to center the block
        assert!(svg.contains("<tspan x=\"10\" y=\"20\" dy=\"-0.6em\">Line 1</tspan>"));
        assert!(svg.contains("<tspan x=\"10\" dy=\"1.2em\">Line 2</tspan>"));
        assert!(!svg.contains("<br/>"));
    }

    #[test]
    fn text_three_lines_centers_vertically() {
        let element = SvgElement::Text {
            x: 50.0,
            y: 100.0,
            content: "Line 1<br/>Line 2<br/>Line 3".to_string(),
            attrs: Attrs::new(),
        };
        let svg = element.to_svg(0);

        // For 3 lines, first line should be offset by -1.2em (2 * 0.6) to center
        assert!(
            svg.contains("<tspan x=\"50\" y=\"100\" dy=\"-1.2em\">Line 1</tspan>"),
            "First line should have dy=-1.2em for 3 lines. Got: {}",
            svg
        );
        assert!(svg.contains("<tspan x=\"50\" dy=\"1.2em\">Line 2</tspan>"));
        assert!(svg.contains("<tspan x=\"50\" dy=\"1.2em\">Line 3</tspan>"));
    }
}
