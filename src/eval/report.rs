//! Report generation for evaluation results.
//!
//! Supports multiple output formats:
//! - Text summary (terminal output)
//! - JSON (machine-readable)
//! - HTML (visual comparison report)
//! - PNG (side-by-side comparison images for AI review)

use super::{EvalResult, Level, Status, TypeStats};
use std::fs;
use std::path::Path;

/// Generate a text summary report
pub fn text_summary(result: &EvalResult) -> String {
    let mut output = String::new();

    // Header
    output.push_str("Selkie Evaluation Report\n");
    output.push_str("========================\n\n");

    // Overall stats
    output.push_str(&format!(
        "Overall Parity: {:.1}% ({}/{} diagrams match reference)\n",
        result.parity_percent, result.matching, result.total
    ));

    if result.avg_visual_similarity > 0.0 {
        output.push_str(&format!(
            "Average Visual Similarity: {:.1}%\n",
            result.avg_visual_similarity * 100.0
        ));
    }

    output.push('\n');

    // By diagram type
    if !result.by_type.is_empty() {
        output.push_str("By Diagram Type:\n");

        let mut types: Vec<(&String, &TypeStats)> = result.by_type.iter().collect();
        types.sort_by_key(|(name, _)| name.as_str());

        for (dtype, stats) in types {
            let bar = progress_bar(stats.parity_percent, 20);
            let ssim_str = if stats.avg_ssim > 0.0 {
                format!("  SSIM: {:.2}", stats.avg_ssim)
            } else {
                String::new()
            };
            output.push_str(&format!(
                "  {:<12} {}  {:.0}% ({}/{}){}\n",
                dtype, bar, stats.parity_percent, stats.matching, stats.total, ssim_str
            ));
        }

        output.push('\n');
    }

    // Issues summary
    output.push_str("Issues Summary:\n");
    output.push_str(&format!(
        "  {:>3} Error    - Structural breaks\n",
        result.issue_counts.errors
    ));
    output.push_str(&format!(
        "  {:>3} Warning  - Significant differences\n",
        result.issue_counts.warnings
    ));
    output.push_str(&format!(
        "  {:>3} Info     - Acceptable variations\n",
        result.issue_counts.info
    ));
    if result.issue_counts.visual_only > 0 {
        output.push_str(&format!(
            "  {:>3} Visual   - Low SSIM (< 0.90) but structural match\n",
            result.issue_counts.visual_only
        ));
    }

    output
}

/// Generate a progress bar string
fn progress_bar(percent: f64, width: usize) -> String {
    let filled = ((percent / 100.0) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

/// Generate a detailed text report with issues
pub fn text_detailed(result: &EvalResult) -> String {
    let mut output = text_summary(result);

    // Show diagrams with issues
    let problem_diagrams: Vec<_> = result
        .diagrams
        .iter()
        .filter(|d| d.status != Status::Match)
        .collect();

    if !problem_diagrams.is_empty() {
        output.push_str(&format!(
            "\nDiagrams with Issues ({}):\n",
            problem_diagrams.len()
        ));
        output.push_str(&"-".repeat(60));
        output.push('\n');

        for diagram in problem_diagrams.iter().take(20) {
            let status_icon = match diagram.status {
                Status::Match => "✓",
                Status::Warning => "⚠",
                Status::Error => "✗",
            };
            output.push_str(&format!("\n[{}] {}\n", status_icon, diagram.name));

            if let Some(ssim) = diagram.visual_similarity {
                output.push_str(&format!("  SSIM: {:.1}%\n", ssim * 100.0));
            }

            for issue in &diagram.issues {
                let level_str = match issue.level {
                    Level::Error => "ERROR",
                    Level::Warning => "WARN",
                    Level::Info => "INFO",
                };
                output.push_str(&format!(
                    "  [{}] {}: {}\n",
                    level_str, issue.check, issue.message
                ));
            }
        }

        if problem_diagrams.len() > 20 {
            output.push_str(&format!(
                "\n... and {} more diagrams with issues\n",
                problem_diagrams.len() - 20
            ));
        }
    }

    output
}

/// Write JSON report to file
pub fn write_json(result: &EvalResult, path: &Path) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(result).map_err(std::io::Error::other)?;
    fs::write(path, json)
}

/// Generate and write HTML comparison report
pub fn write_html(result: &EvalResult, path: &Path) -> std::io::Result<()> {
    let html = generate_html(result);
    fs::write(path, html)
}

/// Generate HTML report content
fn generate_html(result: &EvalResult) -> String {
    let mut html = String::from(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Selkie Evaluation Report</title>
    <style>
        * { box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            margin: 0;
            padding: 20px;
            background: #f5f5f5;
        }
        h1 { color: #333; margin-bottom: 10px; }
        .summary {
            background: white;
            padding: 20px;
            border-radius: 8px;
            margin-bottom: 20px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
        .summary-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
            gap: 15px;
            margin-top: 15px;
        }
        .stat-box {
            background: #f8f9fa;
            padding: 15px;
            border-radius: 6px;
            text-align: center;
        }
        .stat-value { font-size: 2em; font-weight: bold; color: #333; }
        .stat-label { color: #666; font-size: 0.9em; }
        .diagram-card {
            background: white;
            border-radius: 8px;
            margin-bottom: 20px;
            overflow: hidden;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
        .diagram-header {
            padding: 15px 20px;
            border-bottom: 1px solid #eee;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }
        .diagram-name { font-size: 1.2em; font-weight: bold; }
        .status-badge {
            padding: 5px 12px;
            border-radius: 20px;
            font-weight: bold;
            font-size: 0.9em;
        }
        .status-match { background: #d4edda; color: #155724; }
        .status-warning { background: #fff3cd; color: #856404; }
        .status-error { background: #f8d7da; color: #721c24; }
        .issues-panel {
            padding: 15px 20px;
            border-top: 1px solid #eee;
        }
        .issue {
            padding: 8px 12px;
            margin: 5px 0;
            border-radius: 4px;
            font-size: 0.9em;
        }
        .issue-error { background: #f8d7da; }
        .issue-warning { background: #fff3cd; }
        .issue-info { background: #d1ecf1; }
        .no-issues { color: #28a745; font-style: italic; }
        .diagram-compare {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
            gap: 16px;
            padding: 16px 20px 0;
        }
        .diagram-panel {
            border: 1px solid #e5e7eb;
            border-radius: 8px;
            overflow: hidden;
            background: #fff;
        }
        .panel-header {
            padding: 8px 12px;
            font-weight: 600;
            color: #374151;
            border-bottom: 1px solid #e5e7eb;
            background: #f9fafb;
        }
        .panel-header.rs { background: #fef3c7; color: #92400e; }
        .panel-header.ref { background: #dbeafe; color: #1e40af; }
        .panel-content {
            padding: 12px;
            min-height: 140px;
            display: flex;
            align-items: center;
            justify-content: center;
            background: #fff;
        }
        .panel-content svg {
            max-width: 100%;
            height: auto;
        }
        .panel-error {
            color: #dc2626;
            font-style: italic;
        }
        details.diagram-source {
            padding: 0 20px 16px;
        }
        details.diagram-source summary {
            cursor: pointer;
            color: #374151;
            font-weight: 600;
            margin-top: 12px;
        }
        details.diagram-source pre {
            margin-top: 10px;
            background: #1f2937;
            color: #e5e7eb;
            padding: 12px;
            border-radius: 6px;
            font-size: 12px;
            white-space: pre-wrap;
        }
    </style>
</head>
<body>
    <h1>Selkie Evaluation Report</h1>
"#,
    );

    // Summary section
    html.push_str(&format!(
        r#"
    <div class="summary">
        <h2>Summary</h2>
        <div class="summary-grid">
            <div class="stat-box">
                <div class="stat-value">{}</div>
                <div class="stat-label">Diagrams Compared</div>
            </div>
            <div class="stat-box">
                <div class="stat-value">{:.0}%</div>
                <div class="stat-label">Parity</div>
            </div>
            <div class="stat-box">
                <div class="stat-value">{}</div>
                <div class="stat-label">Matching</div>
            </div>
            <div class="stat-box">
                <div class="stat-value">{}</div>
                <div class="stat-label">Errors</div>
            </div>
        </div>
    </div>
"#,
        result.total, result.parity_percent, result.matching, result.issue_counts.errors
    ));

    // Individual diagram cards
    for diagram in &result.diagrams {
        let status_class = match diagram.status {
            Status::Match => "status-match",
            Status::Warning => "status-warning",
            Status::Error => "status-error",
        };
        let status_text = match diagram.status {
            Status::Match => "Match",
            Status::Warning => "Warning",
            Status::Error => "Error",
        };

        html.push_str(&format!(
            r#"
    <div class="diagram-card">
        <div class="diagram-header">
            <span class="diagram-name">{}</span>
            <span class="status-badge {}">{}</span>
        </div>
"#,
            html_escape(&diagram.name),
            status_class,
            status_text
        ));

        html.push_str(r#"<div class="diagram-compare">"#);
        html.push_str(&render_svg_panel(
            "selkie",
            "rs",
            diagram.selkie_svg.as_deref(),
        ));
        html.push_str(&render_svg_panel(
            "mermaid.js",
            "ref",
            diagram.reference_svg.as_deref(),
        ));
        html.push_str("</div>");

        if let Some(ref source) = diagram.diagram_text {
            html.push_str(&format!(
                r#"
        <details class="diagram-source">
            <summary>Source</summary>
            <pre>{}</pre>
        </details>
"#,
                html_escape(source)
            ));
        }

        // Issues
        html.push_str(r#"        <div class="issues-panel">"#);
        if diagram.issues.is_empty() {
            html.push_str(r#"<p class="no-issues">No issues detected</p>"#);
        } else {
            for issue in &diagram.issues {
                let issue_class = match issue.level {
                    Level::Error => "issue-error",
                    Level::Warning => "issue-warning",
                    Level::Info => "issue-info",
                };
                let level_text = match issue.level {
                    Level::Error => "ERROR",
                    Level::Warning => "WARN",
                    Level::Info => "INFO",
                };
                html.push_str(&format!(
                    r#"<div class="issue {}"><strong>[{}]</strong> {}: {}</div>"#,
                    issue_class,
                    level_text,
                    html_escape(&issue.check),
                    html_escape(&issue.message)
                ));
            }
        }
        html.push_str("</div></div>");
    }

    html.push_str("</body></html>");
    html
}

/// Escape HTML special characters
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn render_svg_panel(label: &str, class_name: &str, svg: Option<&str>) -> String {
    let content = if let Some(svg) = svg {
        let fragment = svg_fragment(svg);
        format!(r#"<div class="panel-content">{}</div>"#, fragment)
    } else {
        r#"<div class="panel-content"><span class="panel-error">SVG not available</span></div>"#
            .to_string()
    };

    format!(
        r#"
        <div class="diagram-panel">
            <div class="panel-header {}">{}</div>
            {}
        </div>
"#,
        class_name,
        html_escape(label),
        content
    )
}

fn svg_fragment(svg: &str) -> &str {
    if let Some(pos) = svg.find("<svg") {
        &svg[pos..]
    } else {
        svg
    }
}

/// PNG manifest for AI review
#[derive(Debug, serde::Serialize)]
pub struct PngManifest {
    pub diagrams: Vec<PngManifestEntry>,
}

/// Entry in PNG manifest
#[derive(Debug, serde::Serialize)]
pub struct PngManifestEntry {
    pub name: String,
    pub diagram_type: String,
    pub png: String,
    pub structural_match: bool,
    pub visual_similarity: Option<f64>,
    pub issues: Vec<String>,
}

/// Write PNG comparison images and manifest
///
/// This creates side-by-side PNG images for AI review.
/// Requires the `png` feature for SVG to PNG conversion.
pub fn write_pngs(
    _result: &EvalResult,
    _output_dir: &Path,
    _selkie_svgs: &[(String, String)],    // (name, svg)
    _reference_svgs: &[(String, String)], // (name, svg)
) -> std::io::Result<()> {
    // TODO: Implement PNG generation when resvg is available
    // For now, this is a placeholder

    eprintln!("PNG generation requires the 'png' feature");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::IssueCounts;
    use std::collections::HashMap;

    fn make_test_result() -> EvalResult {
        EvalResult {
            total: 10,
            matching: 8,
            parity_percent: 80.0,
            avg_visual_similarity: 0.92,
            by_type: HashMap::from([
                (
                    "flowchart".to_string(),
                    TypeStats {
                        total: 5,
                        matching: 4,
                        parity_percent: 80.0,
                        avg_ssim: 0.93,
                    },
                ),
                (
                    "pie".to_string(),
                    TypeStats {
                        total: 5,
                        matching: 4,
                        parity_percent: 80.0,
                        avg_ssim: 0.91,
                    },
                ),
            ]),
            issue_counts: IssueCounts {
                errors: 2,
                warnings: 5,
                info: 3,
                visual_only: 1,
            },
            diagrams: vec![],
        }
    }

    #[test]
    fn test_text_summary() {
        let result = make_test_result();
        let summary = text_summary(&result);
        assert!(summary.contains("80.0%"));
        assert!(summary.contains("8/10"));
        assert!(summary.contains("flowchart"));
    }

    #[test]
    fn test_progress_bar() {
        assert_eq!(progress_bar(100.0, 10), "██████████");
        assert_eq!(progress_bar(50.0, 10), "█████░░░░░");
        assert_eq!(progress_bar(0.0, 10), "░░░░░░░░░░");
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a & b"), "a &amp; b");
    }
}
