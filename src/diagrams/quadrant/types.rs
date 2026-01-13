//! Quadrant chart diagram types
//!
//! Quadrant charts divide data into four quadrants based on two axes,
//! commonly used for prioritization matrices.

use std::collections::HashMap;

/// Point styling options
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PointStyle {
    /// Point radius
    pub radius: Option<f64>,
    /// Fill color (hex code)
    pub color: Option<String>,
    /// Stroke color (hex code)
    pub stroke_color: Option<String>,
    /// Stroke width (e.g., "10px")
    pub stroke_width: Option<String>,
}

/// A point in the quadrant chart
#[derive(Debug, Clone, PartialEq)]
pub struct QuadrantPoint {
    /// Point label text
    pub text: String,
    /// X coordinate (0.0 to 1.0)
    pub x: f64,
    /// Y coordinate (0.0 to 1.0)
    pub y: f64,
    /// CSS class name
    pub class_name: Option<String>,
    /// Point styling
    pub style: PointStyle,
}

impl QuadrantPoint {
    /// Create a new point
    pub fn new(text: String, x: f64, y: f64) -> Self {
        Self {
            text,
            x,
            y,
            class_name: None,
            style: PointStyle::default(),
        }
    }
}

/// A class definition for styling
#[derive(Debug, Clone, PartialEq)]
pub struct ClassDef {
    pub name: String,
    pub styles: Vec<String>,
}

/// The Quadrant Chart database that stores all diagram data
#[derive(Debug, Clone, Default)]
pub struct QuadrantDb {
    /// Diagram title
    pub title: String,
    /// X-axis left label
    pub x_axis_left: String,
    /// X-axis right label
    pub x_axis_right: String,
    /// Y-axis bottom label
    pub y_axis_bottom: String,
    /// Y-axis top label
    pub y_axis_top: String,
    /// Quadrant 1 (top-right) label
    pub quadrant1: String,
    /// Quadrant 2 (top-left) label
    pub quadrant2: String,
    /// Quadrant 3 (bottom-left) label
    pub quadrant3: String,
    /// Quadrant 4 (bottom-right) label
    pub quadrant4: String,
    /// Points in the chart
    points: Vec<QuadrantPoint>,
    /// Class definitions
    classes: HashMap<String, ClassDef>,
}

impl QuadrantDb {
    /// Create a new empty QuadrantDb
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all data
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Set the diagram title
    pub fn set_diagram_title(&mut self, title: &str) {
        self.title = title.to_string();
    }

    /// Set X-axis left text
    pub fn set_x_axis_left_text(&mut self, text: &str) {
        self.x_axis_left = text.to_string();
    }

    /// Set X-axis right text
    pub fn set_x_axis_right_text(&mut self, text: &str) {
        self.x_axis_right = text.to_string();
    }

    /// Set Y-axis bottom text
    pub fn set_y_axis_bottom_text(&mut self, text: &str) {
        self.y_axis_bottom = text.to_string();
    }

    /// Set Y-axis top text
    pub fn set_y_axis_top_text(&mut self, text: &str) {
        self.y_axis_top = text.to_string();
    }

    /// Set quadrant 1 text (top-right)
    pub fn set_quadrant1_text(&mut self, text: &str) {
        self.quadrant1 = text.to_string();
    }

    /// Set quadrant 2 text (top-left)
    pub fn set_quadrant2_text(&mut self, text: &str) {
        self.quadrant2 = text.to_string();
    }

    /// Set quadrant 3 text (bottom-left)
    pub fn set_quadrant3_text(&mut self, text: &str) {
        self.quadrant3 = text.to_string();
    }

    /// Set quadrant 4 text (bottom-right)
    pub fn set_quadrant4_text(&mut self, text: &str) {
        self.quadrant4 = text.to_string();
    }

    /// Add a point to the chart
    pub fn add_point(&mut self, text: &str, class_name: &str, x: &str, y: &str, styles: &[&str]) {
        let x: f64 = x.parse().unwrap_or(0.0);
        let y: f64 = y.parse().unwrap_or(0.0);

        // Validate coordinates
        if !(0.0..=1.0).contains(&x) || !(0.0..=1.0).contains(&y) {
            panic!("Point coordinates must be between 0 and 1");
        }

        let mut point = QuadrantPoint::new(text.to_string(), x, y);

        if !class_name.is_empty() {
            point.class_name = Some(class_name.to_string());
        }

        // Parse styles
        point.style = self.parse_styles(styles);

        self.points.push(point);
    }

    /// Parse styles from string array
    pub fn parse_styles(&self, styles: &[&str]) -> PointStyle {
        let mut result = PointStyle::default();

        for style in styles {
            let parts: Vec<&str> = style.splitn(2, ':').collect();
            if parts.len() != 2 {
                continue;
            }

            let name = parts[0].trim();
            let value = parts[1].trim();

            match name {
                "radius" => {
                    if let Ok(r) = value.parse::<f64>() {
                        result.radius = Some(r);
                    } else {
                        panic!(
                            "value for radius {} is invalid, please use a valid number",
                            value
                        );
                    }
                }
                "color" => {
                    if self.is_valid_hex(value) {
                        result.color = Some(value.to_string());
                    } else {
                        panic!(
                            "value for color {} is invalid, please use a valid hex code",
                            value
                        );
                    }
                }
                "stroke-color" => {
                    if self.is_valid_hex(value) {
                        result.stroke_color = Some(value.to_string());
                    } else {
                        panic!(
                            "value for stroke-color {} is invalid, please use a valid hex code",
                            value
                        );
                    }
                }
                "stroke-width" => {
                    if self.is_valid_pixels(value) {
                        result.stroke_width = Some(value.to_string());
                    } else {
                        panic!(
                            "value for stroke-width {} is invalid, please use a valid number of pixels (eg. 10px)",
                            value
                        );
                    }
                }
                _ => {
                    panic!("style named {} is not supported.", name);
                }
            }
        }

        result
    }

    /// Validate hex color code
    fn is_valid_hex(&self, value: &str) -> bool {
        if !value.starts_with('#') {
            return false;
        }
        let hex = &value[1..];
        (hex.len() == 3 || hex.len() == 6) && hex.chars().all(|c| c.is_ascii_hexdigit())
    }

    /// Validate pixel value
    fn is_valid_pixels(&self, value: &str) -> bool {
        if !value.ends_with("px") {
            return false;
        }
        let num = &value[..value.len() - 2];
        num.parse::<f64>().is_ok()
    }

    /// Add a class definition
    pub fn add_class(&mut self, name: &str, styles: &[&str]) {
        let class_def = ClassDef {
            name: name.to_string(),
            styles: styles.iter().map(|s| s.to_string()).collect(),
        };
        self.classes.insert(name.to_string(), class_def);
    }

    /// Get all points
    pub fn get_points(&self) -> &[QuadrantPoint] {
        &self.points
    }

    /// Get a class definition
    pub fn get_class(&self, name: &str) -> Option<&ClassDef> {
        self.classes.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================
    // Style parsing tests
    // ==================

    #[test]
    fn test_parse_styles_all_params() {
        let db = QuadrantDb::new();
        let styles = vec![
            "radius: 10",
            "color: #ff0000",
            "stroke-color: #ff00ff",
            "stroke-width: 10px",
        ];
        let result = db.parse_styles(&styles);

        assert_eq!(result.radius, Some(10.0));
        assert_eq!(result.color, Some("#ff0000".to_string()));
        assert_eq!(result.stroke_color, Some("#ff00ff".to_string()));
        assert_eq!(result.stroke_width, Some("10px".to_string()));
    }

    #[test]
    #[should_panic(expected = "style named test_name is not supported")]
    fn test_parse_styles_unsupported_name() {
        let db = QuadrantDb::new();
        let styles = vec!["test_name: value"];
        db.parse_styles(&styles);
    }

    #[test]
    fn test_parse_styles_empty() {
        let db = QuadrantDb::new();
        let styles: Vec<&str> = vec![];
        let result = db.parse_styles(&styles);

        assert_eq!(result, PointStyle::default());
    }

    #[test]
    #[should_panic(expected = "value for radius f is invalid")]
    fn test_parse_styles_invalid_radius() {
        let db = QuadrantDb::new();
        let styles = vec!["radius: f"];
        db.parse_styles(&styles);
    }

    #[test]
    #[should_panic(expected = "value for color ffaa is invalid")]
    fn test_parse_styles_invalid_color() {
        let db = QuadrantDb::new();
        let styles = vec!["color: ffaa"];
        db.parse_styles(&styles);
    }

    #[test]
    #[should_panic(expected = "value for stroke-color #f677779 is invalid")]
    fn test_parse_styles_invalid_stroke_color() {
        let db = QuadrantDb::new();
        let styles = vec!["stroke-color: #f677779"];
        db.parse_styles(&styles);
    }

    #[test]
    #[should_panic(expected = "value for stroke-width 30 is invalid")]
    fn test_parse_styles_invalid_stroke_width() {
        let db = QuadrantDb::new();
        let styles = vec!["stroke-width: 30"];
        db.parse_styles(&styles);
    }

    // ==================
    // Axis and quadrant tests
    // ==================

    #[test]
    fn test_set_x_axis_text() {
        let mut db = QuadrantDb::new();
        db.set_x_axis_left_text("urgent");
        db.set_x_axis_right_text("not urgent");

        assert_eq!(db.x_axis_left, "urgent");
        assert_eq!(db.x_axis_right, "not urgent");
    }

    #[test]
    fn test_set_y_axis_text() {
        let mut db = QuadrantDb::new();
        db.set_y_axis_bottom_text("Ability to Execute");
        db.set_y_axis_top_text("y-axis-2");

        assert_eq!(db.y_axis_bottom, "Ability to Execute");
        assert_eq!(db.y_axis_top, "y-axis-2");
    }

    #[test]
    fn test_set_quadrant_text() {
        let mut db = QuadrantDb::new();
        db.set_quadrant1_text("Leaders");
        db.set_quadrant2_text("Challengers");
        db.set_quadrant3_text("Niche");
        db.set_quadrant4_text("Visionaries");

        assert_eq!(db.quadrant1, "Leaders");
        assert_eq!(db.quadrant2, "Challengers");
        assert_eq!(db.quadrant3, "Niche");
        assert_eq!(db.quadrant4, "Visionaries");
    }

    #[test]
    fn test_set_title() {
        let mut db = QuadrantDb::new();
        db.set_diagram_title("Analytics and Business Intelligence Platforms");

        assert_eq!(db.title, "Analytics and Business Intelligence Platforms");
    }

    // ==================
    // Point tests
    // ==================

    #[test]
    fn test_add_point_basic() {
        let mut db = QuadrantDb::new();
        db.add_point("point1", "", "0.1", "0.4", &[]);

        let points = db.get_points();
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].text, "point1");
        assert_eq!(points[0].x, 0.1);
        assert_eq!(points[0].y, 0.4);
    }

    #[test]
    fn test_add_point_with_class() {
        let mut db = QuadrantDb::new();
        db.add_point(
            "Salesforce",
            "class1",
            "0.55",
            "0.60",
            &["radius: 10", "color: #ff0000"],
        );

        let points = db.get_points();
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].text, "Salesforce");
        assert_eq!(points[0].class_name, Some("class1".to_string()));
        assert_eq!(points[0].style.radius, Some(10.0));
        assert_eq!(points[0].style.color, Some("#ff0000".to_string()));
    }

    #[test]
    #[should_panic(expected = "Point coordinates must be between 0 and 1")]
    fn test_add_point_invalid_coords() {
        let mut db = QuadrantDb::new();
        db.add_point("point", "", "1.2", "0.4", &[]);
    }

    // ==================
    // Full chart test
    // ==================

    #[test]
    fn test_full_chart() {
        let mut db = QuadrantDb::new();
        db.set_diagram_title("Analytics and Business Intelligence Platforms");
        db.set_x_axis_left_text("Completeness of Vision");
        db.set_x_axis_right_text("x-axis-2");
        db.set_y_axis_bottom_text("Ability to Execute");
        db.set_y_axis_top_text("y-axis-2");
        db.set_quadrant1_text("Leaders");
        db.set_quadrant2_text("Challengers");
        db.set_quadrant3_text("Niche");
        db.set_quadrant4_text("Visionaries");

        db.add_point("Microsoft", "", "0.75", "0.75", &[]);
        db.add_point("Salesforce", "", "0.55", "0.60", &[]);
        db.add_point("IBM", "", "0.51", "0.40", &[]);
        db.add_point("Incorta", "", "0.20", "0.30", &[]);

        let points = db.get_points();
        assert_eq!(points.len(), 4);
        assert_eq!(points[0].text, "Microsoft");
        assert_eq!(points[1].text, "Salesforce");
        assert_eq!(points[2].text, "IBM");
        assert_eq!(points[3].text, "Incorta");
    }

    #[test]
    fn test_chart_with_styling() {
        let mut db = QuadrantDb::new();
        db.add_point(
            "Microsoft",
            "",
            "0.75",
            "0.75",
            &[
                "stroke-color: #ff00ff",
                "stroke-width: 10px",
                "color: #ff0000",
                "radius: 10",
            ],
        );
        db.add_point(
            "Salesforce",
            "class1",
            "0.55",
            "0.60",
            &["radius: 10", "color: #ff0000"],
        );

        let points = db.get_points();

        // First point
        assert_eq!(points[0].style.stroke_color, Some("#ff00ff".to_string()));
        assert_eq!(points[0].style.stroke_width, Some("10px".to_string()));
        assert_eq!(points[0].style.color, Some("#ff0000".to_string()));
        assert_eq!(points[0].style.radius, Some(10.0));

        // Second point with class
        assert_eq!(points[1].class_name, Some("class1".to_string()));
        assert_eq!(points[1].style.radius, Some(10.0));
        assert_eq!(points[1].style.color, Some("#ff0000".to_string()));
    }

    // ==================
    // Class definition tests
    // ==================

    #[test]
    fn test_add_class() {
        let mut db = QuadrantDb::new();
        db.add_class("constructor", &["fill:#ff0000"]);

        let class = db.get_class("constructor");
        assert!(class.is_some());
        assert_eq!(class.unwrap().styles, vec!["fill:#ff0000"]);
    }

    // ==================
    // Clear test
    // ==================

    #[test]
    fn test_clear() {
        let mut db = QuadrantDb::new();
        db.set_diagram_title("Test");
        db.add_point("point1", "", "0.5", "0.5", &[]);
        db.clear();

        assert_eq!(db.title, "");
        assert!(db.get_points().is_empty());
    }
}
