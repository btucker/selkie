//! Evaluation module for comparing selkie output against mermaid.js reference.
//!
//! This module provides tools to evaluate selkie's parity with mermaid.js:
//! - Structural comparison (node/edge counts, labels)
//! - Visual similarity using SSIM
//! - Report generation (text, JSON, HTML, PNG)
//! - Reference SVG caching

pub mod cache;
pub mod checks;
pub mod png;
pub mod report;
pub mod runner;
pub mod samples;
pub mod ssim;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of evaluating multiple diagrams
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalResult {
    /// Total number of diagrams evaluated
    pub total: usize,
    /// Number of diagrams that match reference (no errors)
    pub matching: usize,
    /// Overall parity percentage
    pub parity_percent: f64,
    /// Average SSIM visual similarity
    pub avg_visual_similarity: f64,
    /// Results grouped by diagram type
    pub by_type: HashMap<String, TypeStats>,
    /// Issue counts by level
    pub issue_counts: IssueCounts,
    /// Individual diagram results
    pub diagrams: Vec<DiagramResult>,
}

/// Statistics for a diagram type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeStats {
    pub total: usize,
    pub matching: usize,
    pub parity_percent: f64,
    pub avg_ssim: f64,
}

/// Counts of issues by level
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IssueCounts {
    pub errors: usize,
    pub warnings: usize,
    pub info: usize,
    /// Diagrams with low SSIM but structural match
    pub visual_only: usize,
}

/// Result of evaluating a single diagram
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramResult {
    /// Diagram name/identifier
    pub name: String,
    /// Source file (if known)
    pub source: Option<String>,
    /// Detected diagram type
    pub diagram_type: String,
    /// The diagram source text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagram_text: Option<String>,
    /// Overall status
    pub status: Status,
    /// SSIM visual similarity score (0-1)
    pub visual_similarity: Option<f64>,
    /// Structural match (no errors)
    pub structural_match: bool,
    /// Issues found during evaluation
    pub issues: Vec<Issue>,
    /// Parse result
    pub parse_result: ParseResult,
    /// Render result (if parsed successfully)
    pub render_result: Option<RenderResult>,
    /// Selkie-rendered SVG for HTML reports
    #[serde(skip_serializing, skip_deserializing)]
    pub selkie_svg: Option<String>,
    /// Reference SVG for HTML reports
    #[serde(skip_serializing, skip_deserializing)]
    pub reference_svg: Option<String>,
}

/// Parse result for a diagram
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseResult {
    /// Whether selkie successfully parsed the diagram
    pub selkie_success: bool,
    /// Selkie parse error (if any)
    pub selkie_error: Option<String>,
    /// Whether mermaid.js successfully parsed the diagram
    pub reference_success: bool,
    /// Mermaid.js parse error (if any)
    pub reference_error: Option<String>,
}

/// Render result for a diagram
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderResult {
    /// Whether selkie successfully rendered the diagram
    pub selkie_success: bool,
    /// Selkie render error (if any)
    pub selkie_error: Option<String>,
    /// Whether mermaid.js successfully rendered the diagram
    pub reference_success: bool,
    /// Mermaid.js render error (if any)
    pub reference_error: Option<String>,
    /// Selkie SVG dimensions
    pub selkie_dimensions: Option<Dimensions>,
    /// Reference SVG dimensions
    pub reference_dimensions: Option<Dimensions>,
}

/// SVG dimensions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dimensions {
    pub width: f64,
    pub height: f64,
}

/// Overall status of a diagram evaluation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// No errors, structural match
    Match,
    /// Has warnings but no errors
    Warning,
    /// Has errors (structural breaks)
    Error,
}

/// Issue severity level (3-level system)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    /// Structural break - diagram is functionally wrong
    Error,
    /// Significant difference - diagram may look noticeably different
    Warning,
    /// Acceptable variation - likely intentional implementation difference
    Info,
}

/// An issue found during evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    /// Severity level
    pub level: Level,
    /// Category of the check (e.g., "node_count", "dimensions", "labels")
    pub check: String,
    /// Human-readable description
    pub message: String,
    /// Expected value (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    /// Actual value (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,
}

impl Issue {
    /// Create a new error-level issue
    pub fn error(check: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            level: Level::Error,
            check: check.into(),
            message: message.into(),
            expected: None,
            actual: None,
        }
    }

    /// Create a new warning-level issue
    pub fn warning(check: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            level: Level::Warning,
            check: check.into(),
            message: message.into(),
            expected: None,
            actual: None,
        }
    }

    /// Create a new info-level issue
    pub fn info(check: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            level: Level::Info,
            check: check.into(),
            message: message.into(),
            expected: None,
            actual: None,
        }
    }

    /// Add expected/actual values
    pub fn with_values(mut self, expected: impl Into<String>, actual: impl Into<String>) -> Self {
        self.expected = Some(expected.into());
        self.actual = Some(actual.into());
        self
    }
}

impl DiagramResult {
    /// Check if this diagram has any errors
    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.level == Level::Error)
    }

    /// Check if this diagram has any warnings
    pub fn has_warnings(&self) -> bool {
        self.issues.iter().any(|i| i.level == Level::Warning)
    }

    /// Determine status from issues
    pub fn determine_status(&self) -> Status {
        if self.has_errors() {
            Status::Error
        } else if self.has_warnings() {
            Status::Warning
        } else {
            Status::Match
        }
    }
}

impl EvalResult {
    /// Create a new empty result
    pub fn new() -> Self {
        Self {
            total: 0,
            matching: 0,
            parity_percent: 0.0,
            avg_visual_similarity: 0.0,
            by_type: HashMap::new(),
            issue_counts: IssueCounts::default(),
            diagrams: Vec::new(),
        }
    }

    /// Compute summary statistics from diagram results
    pub fn compute_stats(&mut self) {
        self.total = self.diagrams.len();
        self.matching = self
            .diagrams
            .iter()
            .filter(|d| d.status == Status::Match)
            .count();
        self.parity_percent = if self.total > 0 {
            100.0 * self.matching as f64 / self.total as f64
        } else {
            0.0
        };

        // Compute average SSIM
        let ssim_values: Vec<f64> = self
            .diagrams
            .iter()
            .filter_map(|d| d.visual_similarity)
            .collect();
        self.avg_visual_similarity = if ssim_values.is_empty() {
            0.0
        } else {
            ssim_values.iter().sum::<f64>() / ssim_values.len() as f64
        };

        // Count issues
        self.issue_counts = IssueCounts::default();
        for diagram in &self.diagrams {
            for issue in &diagram.issues {
                match issue.level {
                    Level::Error => self.issue_counts.errors += 1,
                    Level::Warning => self.issue_counts.warnings += 1,
                    Level::Info => self.issue_counts.info += 1,
                }
            }
            // Count diagrams with low SSIM but structural match
            if diagram.structural_match {
                if let Some(ssim) = diagram.visual_similarity {
                    if ssim < 0.90 {
                        self.issue_counts.visual_only += 1;
                    }
                }
            }
        }

        // Group by type
        self.by_type.clear();
        for diagram in &self.diagrams {
            let entry = self
                .by_type
                .entry(diagram.diagram_type.clone())
                .or_insert(TypeStats {
                    total: 0,
                    matching: 0,
                    parity_percent: 0.0,
                    avg_ssim: 0.0,
                });
            entry.total += 1;
            if diagram.status == Status::Match {
                entry.matching += 1;
            }
        }

        // Compute per-type stats
        for (dtype, stats) in &mut self.by_type {
            stats.parity_percent = if stats.total > 0 {
                100.0 * stats.matching as f64 / stats.total as f64
            } else {
                0.0
            };

            // Compute per-type average SSIM
            let type_ssim: Vec<f64> = self
                .diagrams
                .iter()
                .filter(|d| &d.diagram_type == dtype)
                .filter_map(|d| d.visual_similarity)
                .collect();
            stats.avg_ssim = if type_ssim.is_empty() {
                0.0
            } else {
                type_ssim.iter().sum::<f64>() / type_ssim.len() as f64
            };
        }
    }
}

impl Default for EvalResult {
    fn default() -> Self {
        Self::new()
    }
}
