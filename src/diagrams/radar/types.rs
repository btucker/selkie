//! Radar diagram types
//!
//! Radar diagrams (spider/web charts) show multivariate data as a polygon
//! plotted on axes radiating from a center point.

use std::collections::HashMap;

/// Graticule style (background grid)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Graticule {
    #[default]
    Circle,
    Polygon,
}

impl Graticule {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "polygon" => Graticule::Polygon,
            _ => Graticule::Circle,
        }
    }
}

/// A radar axis
#[derive(Debug, Clone, PartialEq)]
pub struct RadarAxis {
    pub name: String,
    pub label: String,
}

impl RadarAxis {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            label: name.to_string(),
        }
    }

    pub fn with_label(name: &str, label: &str) -> Self {
        Self {
            name: name.to_string(),
            label: label.to_string(),
        }
    }
}

/// A data point entry (value for a specific axis)
#[derive(Debug, Clone, PartialEq)]
pub struct RadarEntry {
    pub axis: Option<String>,
    pub value: f64,
}

/// A radar curve (data series)
#[derive(Debug, Clone, PartialEq)]
pub struct RadarCurve {
    pub name: String,
    pub label: String,
    pub entries: Vec<f64>,
}

impl RadarCurve {
    pub fn new(name: &str, entries: Vec<f64>) -> Self {
        Self {
            name: name.to_string(),
            label: name.to_string(),
            entries,
        }
    }

    pub fn with_label(name: &str, label: &str, entries: Vec<f64>) -> Self {
        Self {
            name: name.to_string(),
            label: label.to_string(),
            entries,
        }
    }
}

/// Radar chart options
#[derive(Debug, Clone, PartialEq)]
pub struct RadarOptions {
    pub show_legend: bool,
    pub ticks: usize,
    pub max: Option<f64>,
    pub min: f64,
    pub graticule: Graticule,
}

impl Default for RadarOptions {
    fn default() -> Self {
        Self {
            show_legend: true,
            ticks: 5,
            max: None,
            min: 0.0,
            graticule: Graticule::Circle,
        }
    }
}

/// The Radar database
#[derive(Debug, Clone, Default)]
pub struct RadarDb {
    title: String,
    acc_title: String,
    acc_description: String,
    axes: Vec<RadarAxis>,
    curves: Vec<RadarCurve>,
    options: RadarOptions,
}

impl RadarDb {
    /// Create a new empty RadarDb
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all data
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Set the diagram title
    pub fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
    }

    /// Get the diagram title
    pub fn get_title(&self) -> &str {
        &self.title
    }

    /// Set the accessibility title
    pub fn set_acc_title(&mut self, title: &str) {
        self.acc_title = title.to_string();
    }

    /// Get the accessibility title
    pub fn get_acc_title(&self) -> &str {
        &self.acc_title
    }

    /// Set the accessibility description
    pub fn set_acc_description(&mut self, description: &str) {
        self.acc_description = description.to_string();
    }

    /// Get the accessibility description
    pub fn get_acc_description(&self) -> &str {
        &self.acc_description
    }

    /// Add an axis
    pub fn add_axis(&mut self, axis: RadarAxis) {
        self.axes.push(axis);
    }

    /// Get all axes
    pub fn get_axes(&self) -> &[RadarAxis] {
        &self.axes
    }

    /// Add a curve with simple numeric entries
    pub fn add_curve(&mut self, name: &str, label: Option<&str>, entries: Vec<f64>) {
        let curve = match label {
            Some(l) => RadarCurve::with_label(name, l, entries),
            None => RadarCurve::new(name, entries),
        };
        self.curves.push(curve);
    }

    /// Add a curve with detailed (axis-referenced) entries
    pub fn add_curve_with_axis_refs(
        &mut self,
        name: &str,
        label: Option<&str>,
        entries: Vec<RadarEntry>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Build a map of axis name -> index
        let axis_indices: HashMap<&str, usize> = self
            .axes
            .iter()
            .enumerate()
            .map(|(i, a)| (a.name.as_str(), i))
            .collect();

        // Order entries according to axes
        let mut ordered_values = vec![0.0; self.axes.len()];
        for entry in entries {
            if let Some(axis_name) = &entry.axis {
                if let Some(&idx) = axis_indices.get(axis_name.as_str()) {
                    ordered_values[idx] = entry.value;
                } else {
                    return Err(format!("Unknown axis: {}", axis_name).into());
                }
            }
        }

        let curve = match label {
            Some(l) => RadarCurve::with_label(name, l, ordered_values),
            None => RadarCurve::new(name, ordered_values),
        };
        self.curves.push(curve);
        Ok(())
    }

    /// Get all curves
    pub fn get_curves(&self) -> &[RadarCurve] {
        &self.curves
    }

    /// Set an option
    pub fn set_option(&mut self, name: &str, value: &str) {
        match name {
            "showLegend" => {
                self.options.show_legend = value.parse().unwrap_or(true);
            }
            "ticks" => {
                if let Ok(v) = value.parse() {
                    self.options.ticks = v;
                }
            }
            "max" => {
                if let Ok(v) = value.parse() {
                    self.options.max = Some(v);
                }
            }
            "min" => {
                if let Ok(v) = value.parse() {
                    self.options.min = v;
                }
            }
            "graticule" => {
                self.options.graticule = Graticule::from_str(value);
            }
            _ => {}
        }
    }

    /// Get options
    pub fn get_options(&self) -> &RadarOptions {
        &self.options
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_db() {
        let db = RadarDb::new();
        assert!(db.get_axes().is_empty());
        assert!(db.get_curves().is_empty());
    }

    #[test]
    fn test_add_axis() {
        let mut db = RadarDb::new();
        db.add_axis(RadarAxis::new("A"));
        db.add_axis(RadarAxis::with_label("B", "Axis B"));

        let axes = db.get_axes();
        assert_eq!(axes.len(), 2);
        assert_eq!(axes[0].name, "A");
        assert_eq!(axes[0].label, "A");
        assert_eq!(axes[1].name, "B");
        assert_eq!(axes[1].label, "Axis B");
    }

    #[test]
    fn test_add_curve() {
        let mut db = RadarDb::new();
        db.add_curve("mycurve", None, vec![1.0, 2.0, 3.0]);

        let curves = db.get_curves();
        assert_eq!(curves.len(), 1);
        assert_eq!(curves[0].name, "mycurve");
        assert_eq!(curves[0].label, "mycurve");
        assert_eq!(curves[0].entries, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_add_curve_with_label() {
        let mut db = RadarDb::new();
        db.add_curve("mycurve", Some("My Curve"), vec![1.0, 2.0, 3.0]);

        let curves = db.get_curves();
        assert_eq!(curves[0].label, "My Curve");
    }

    #[test]
    fn test_add_curve_with_axis_refs() {
        let mut db = RadarDb::new();
        db.add_axis(RadarAxis::new("A"));
        db.add_axis(RadarAxis::new("B"));
        db.add_axis(RadarAxis::new("C"));

        let entries = vec![
            RadarEntry {
                axis: Some("C".to_string()),
                value: 3.0,
            },
            RadarEntry {
                axis: Some("A".to_string()),
                value: 1.0,
            },
            RadarEntry {
                axis: Some("B".to_string()),
                value: 2.0,
            },
        ];

        db.add_curve_with_axis_refs("mycurve", None, entries)
            .unwrap();

        let curves = db.get_curves();
        // Values should be ordered by axis order: A, B, C
        assert_eq!(curves[0].entries, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_set_options() {
        let mut db = RadarDb::new();
        db.set_option("showLegend", "false");
        db.set_option("ticks", "10");
        db.set_option("min", "1");
        db.set_option("max", "10");
        db.set_option("graticule", "polygon");

        let options = db.get_options();
        assert!(!options.show_legend);
        assert_eq!(options.ticks, 10);
        assert_eq!(options.min, 1.0);
        assert_eq!(options.max, Some(10.0));
        assert_eq!(options.graticule, Graticule::Polygon);
    }

    #[test]
    fn test_default_options() {
        let db = RadarDb::new();
        let options = db.get_options();
        assert!(options.show_legend);
        assert_eq!(options.ticks, 5);
        assert_eq!(options.min, 0.0);
        assert_eq!(options.max, None);
        assert_eq!(options.graticule, Graticule::Circle);
    }

    #[test]
    fn test_clear() {
        let mut db = RadarDb::new();
        db.set_title("Test");
        db.add_axis(RadarAxis::new("A"));
        db.add_curve("c", None, vec![1.0]);

        db.clear();

        assert!(db.get_title().is_empty());
        assert!(db.get_axes().is_empty());
        assert!(db.get_curves().is_empty());
    }
}
