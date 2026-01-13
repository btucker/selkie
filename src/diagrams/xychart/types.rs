//! XY Chart diagram types
//!
//! XY charts show data on a 2D coordinate system with line and bar plots.

/// Chart orientation
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ChartOrientation {
    #[default]
    Vertical,
    Horizontal,
}

/// Axis type
#[derive(Debug, Clone, PartialEq)]
pub enum AxisType {
    /// Band axis for categorical data
    Band,
    /// Linear axis for numeric ranges
    Linear,
}

/// Band axis data (categorical)
#[derive(Debug, Clone, PartialEq)]
pub struct BandAxisData {
    pub title: String,
    pub categories: Vec<String>,
}

/// Linear axis data (numeric range)
#[derive(Debug, Clone, PartialEq)]
pub struct LinearAxisData {
    pub title: String,
    pub min: f64,
    pub max: f64,
}

/// X-axis configuration
#[derive(Debug, Clone, PartialEq)]
pub enum XAxisData {
    Band(BandAxisData),
    Linear(LinearAxisData),
}

/// Y-axis configuration
#[derive(Debug, Clone, PartialEq)]
pub enum YAxisData {
    Linear(LinearAxisData),
}

/// Plot type
#[derive(Debug, Clone, PartialEq)]
pub enum PlotType {
    Line,
    Bar,
}

/// A data point
#[derive(Debug, Clone, PartialEq)]
pub struct DataPoint {
    pub label: String,
    pub value: f64,
}

/// A plot (line or bar)
#[derive(Debug, Clone, PartialEq)]
pub struct Plot {
    pub plot_type: PlotType,
    pub data: Vec<DataPoint>,
}

impl Plot {
    pub fn new_line(data: Vec<DataPoint>) -> Self {
        Self {
            plot_type: PlotType::Line,
            data,
        }
    }

    pub fn new_bar(data: Vec<DataPoint>) -> Self {
        Self {
            plot_type: PlotType::Bar,
            data,
        }
    }
}

/// The XY Chart database
#[derive(Debug, Clone, Default)]
pub struct XYChartDb {
    /// Chart title
    pub title: String,
    /// Chart orientation
    pub orientation: ChartOrientation,
    /// X-axis configuration
    pub x_axis: Option<XAxisData>,
    /// Y-axis configuration
    pub y_axis: Option<YAxisData>,
    /// Plots
    plots: Vec<Plot>,
}

impl XYChartDb {
    /// Create a new empty XYChartDb
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all data
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Set the chart title
    pub fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
    }

    /// Set chart orientation
    pub fn set_orientation(&mut self, orientation: ChartOrientation) {
        self.orientation = orientation;
    }

    /// Set X-axis with band (categorical) data
    pub fn set_x_axis_band(&mut self, title: &str, categories: Vec<String>) {
        self.x_axis = Some(XAxisData::Band(BandAxisData {
            title: title.to_string(),
            categories,
        }));
    }

    /// Set X-axis with linear (numeric) range
    pub fn set_x_axis_linear(&mut self, title: &str, min: f64, max: f64) {
        self.x_axis = Some(XAxisData::Linear(LinearAxisData {
            title: title.to_string(),
            min,
            max,
        }));
    }

    /// Set Y-axis with linear (numeric) range
    pub fn set_y_axis_linear(&mut self, title: &str, min: f64, max: f64) {
        self.y_axis = Some(YAxisData::Linear(LinearAxisData {
            title: title.to_string(),
            min,
            max,
        }));
    }

    /// Add a line plot
    pub fn add_line_plot(&mut self, data: Vec<DataPoint>) {
        self.plots.push(Plot::new_line(data));
    }

    /// Add a bar plot
    pub fn add_bar_plot(&mut self, data: Vec<DataPoint>) {
        self.plots.push(Plot::new_bar(data));
    }

    /// Get all plots
    pub fn get_plots(&self) -> &[Plot] {
        &self.plots
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_title() {
        let mut db = XYChartDb::new();
        db.set_title("Sales Data");
        assert_eq!(db.title, "Sales Data");
    }

    #[test]
    fn test_set_orientation_vertical() {
        let mut db = XYChartDb::new();
        db.set_orientation(ChartOrientation::Vertical);
        assert_eq!(db.orientation, ChartOrientation::Vertical);
    }

    #[test]
    fn test_set_orientation_horizontal() {
        let mut db = XYChartDb::new();
        db.set_orientation(ChartOrientation::Horizontal);
        assert_eq!(db.orientation, ChartOrientation::Horizontal);
    }

    #[test]
    fn test_x_axis_band() {
        let mut db = XYChartDb::new();
        db.set_x_axis_band(
            "Categories",
            vec!["A".to_string(), "B".to_string(), "C".to_string()],
        );

        if let Some(XAxisData::Band(axis)) = &db.x_axis {
            assert_eq!(axis.title, "Categories");
            assert_eq!(axis.categories, vec!["A", "B", "C"]);
        } else {
            panic!("Expected band axis");
        }
    }

    #[test]
    fn test_x_axis_linear() {
        let mut db = XYChartDb::new();
        db.set_x_axis_linear("Time", 0.0, 100.0);

        if let Some(XAxisData::Linear(axis)) = &db.x_axis {
            assert_eq!(axis.title, "Time");
            assert_eq!(axis.min, 0.0);
            assert_eq!(axis.max, 100.0);
        } else {
            panic!("Expected linear axis");
        }
    }

    #[test]
    fn test_y_axis_linear() {
        let mut db = XYChartDb::new();
        db.set_y_axis_linear("Value", 0.0, 500.0);

        if let Some(YAxisData::Linear(axis)) = &db.y_axis {
            assert_eq!(axis.title, "Value");
            assert_eq!(axis.min, 0.0);
            assert_eq!(axis.max, 500.0);
        } else {
            panic!("Expected linear axis");
        }
    }

    #[test]
    fn test_add_line_plot() {
        let mut db = XYChartDb::new();
        let data = vec![
            DataPoint {
                label: "A".to_string(),
                value: 10.0,
            },
            DataPoint {
                label: "B".to_string(),
                value: 20.0,
            },
        ];
        db.add_line_plot(data);

        let plots = db.get_plots();
        assert_eq!(plots.len(), 1);
        assert_eq!(plots[0].plot_type, PlotType::Line);
        assert_eq!(plots[0].data.len(), 2);
    }

    #[test]
    fn test_add_bar_plot() {
        let mut db = XYChartDb::new();
        let data = vec![
            DataPoint {
                label: "Q1".to_string(),
                value: 100.0,
            },
            DataPoint {
                label: "Q2".to_string(),
                value: 150.0,
            },
            DataPoint {
                label: "Q3".to_string(),
                value: 200.0,
            },
        ];
        db.add_bar_plot(data);

        let plots = db.get_plots();
        assert_eq!(plots.len(), 1);
        assert_eq!(plots[0].plot_type, PlotType::Bar);
        assert_eq!(plots[0].data.len(), 3);
    }

    #[test]
    fn test_multiple_plots() {
        let mut db = XYChartDb::new();
        db.add_line_plot(vec![DataPoint {
            label: "A".to_string(),
            value: 10.0,
        }]);
        db.add_bar_plot(vec![DataPoint {
            label: "B".to_string(),
            value: 20.0,
        }]);

        let plots = db.get_plots();
        assert_eq!(plots.len(), 2);
        assert_eq!(plots[0].plot_type, PlotType::Line);
        assert_eq!(plots[1].plot_type, PlotType::Bar);
    }

    #[test]
    fn test_clear() {
        let mut db = XYChartDb::new();
        db.set_title("Test");
        db.add_line_plot(vec![DataPoint {
            label: "A".to_string(),
            value: 10.0,
        }]);
        db.clear();

        assert_eq!(db.title, "");
        assert!(db.get_plots().is_empty());
    }
}
