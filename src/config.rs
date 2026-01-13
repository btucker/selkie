//! Configuration for mermaid diagrams

use serde::{Deserialize, Serialize};

/// Security level for diagram rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SecurityLevel {
    /// Strict security - sanitize all user input
    #[default]
    Strict,
    /// Loose security - allow more HTML
    Loose,
    /// Antiscript - remove scripts but allow HTML
    Antiscript,
    /// Sandbox - run in sandboxed iframe
    Sandbox,
}

/// Flowchart-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowchartConfig {
    /// Whether to use HTML labels
    #[serde(default = "default_html_labels")]
    pub html_labels: bool,
    /// Default interpolation curve for edges
    #[serde(default)]
    pub curve: String,
}

fn default_html_labels() -> bool {
    true
}

impl Default for FlowchartConfig {
    fn default() -> Self {
        Self {
            html_labels: true,
            curve: String::from("basis"),
        }
    }
}

/// Pie chart specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PieConfig {
    /// Text position (0.0 to 1.0)
    #[serde(default = "default_text_position")]
    pub text_position: f64,
}

fn default_text_position() -> f64 {
    0.5
}

impl Default for PieConfig {
    fn default() -> Self {
        Self { text_position: 0.5 }
    }
}

/// Global configuration for mermaid
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    /// Security level
    #[serde(default)]
    pub security_level: SecurityLevel,
    /// Flowchart configuration
    #[serde(default)]
    pub flowchart: FlowchartConfig,
    /// Pie chart configuration
    #[serde(default)]
    pub pie: PieConfig,
}

impl Config {
    /// Create a new config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a config with strict security
    pub fn strict() -> Self {
        Self {
            security_level: SecurityLevel::Strict,
            ..Default::default()
        }
    }
}
