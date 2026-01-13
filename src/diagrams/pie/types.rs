//! Pie chart types

use crate::common::CommonDb;

/// Configuration for pie charts
#[derive(Debug, Clone)]
pub struct PieConfig {
    /// Position of text labels (0.0 to 1.0)
    pub text_position: f64,
}

impl Default for PieConfig {
    fn default() -> Self {
        Self { text_position: 0.5 }
    }
}

/// Database for pie chart data
#[derive(Debug, Clone)]
pub struct PieDb {
    /// Common diagram fields (title, accessibility)
    common: CommonDb,
    /// Pie chart sections in declaration order (label, value)
    sections: Vec<(String, f64)>,
    /// Whether to show data values
    show_data: bool,
    /// Pie chart configuration
    config: PieConfig,
}

impl Default for PieDb {
    fn default() -> Self {
        Self::new()
    }
}

impl PieDb {
    /// Create a new pie chart database
    pub fn new() -> Self {
        Self {
            common: CommonDb::new(),
            sections: Vec::new(),
            show_data: false,
            config: PieConfig::default(),
        }
    }

    /// Clear the database
    pub fn clear(&mut self) {
        self.common.clear();
        self.sections.clear();
        self.show_data = false;
    }

    /// Add a section to the pie chart
    pub fn add_section(
        &mut self,
        label: impl Into<String>,
        value: f64,
    ) -> Result<(), crate::error::MermaidError> {
        let label = label.into();
        if value < 0.0 {
            return Err(crate::error::MermaidError::InvalidValue {
                message: format!(
                    "\"{}\" has invalid value: {}. Negative values are not allowed in pie charts. All slice values must be >= 0.",
                    label, value
                ),
            });
        }

        // Only add if label doesn't already exist (preserve first occurrence)
        if !self.sections.iter().any(|(l, _)| l == &label) {
            self.sections.push((label, value));
        }

        Ok(())
    }

    /// Get all sections in declaration order
    pub fn get_sections(&self) -> &[(String, f64)] {
        &self.sections
    }

    /// Get a section value by label
    pub fn get_section(&self, label: &str) -> Option<f64> {
        self.sections
            .iter()
            .find(|(l, _)| l == label)
            .map(|(_, v)| *v)
    }

    /// Set whether to show data values
    pub fn set_show_data(&mut self, show: bool) {
        self.show_data = show;
    }

    /// Get whether to show data values
    pub fn get_show_data(&self) -> bool {
        self.show_data
    }

    /// Get configuration
    pub fn get_config(&self) -> &PieConfig {
        &self.config
    }

    /// Set diagram title
    pub fn set_diagram_title(&mut self, title: impl Into<String>) {
        self.common.set_diagram_title(title);
    }

    /// Get diagram title
    pub fn get_diagram_title(&self) -> Option<&str> {
        self.common.get_diagram_title()
    }

    /// Set accessibility title
    pub fn set_acc_title(&mut self, title: impl Into<String>) {
        self.common.set_acc_title(title);
    }

    /// Get accessibility title
    pub fn get_acc_title(&self) -> Option<&str> {
        self.common.get_acc_title()
    }

    /// Set accessibility description
    pub fn set_acc_description(&mut self, desc: impl Into<String>) {
        self.common.set_acc_description(desc);
    }

    /// Get accessibility description
    pub fn get_acc_description(&self) -> Option<&str> {
        self.common.get_acc_description()
    }
}
