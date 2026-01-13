//! SVG comparison tool - analyzes differences between mermaid-rs and mermaid.js outputs

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

/// Metrics extracted from an SVG file
#[derive(Debug, Default)]
struct SvgMetrics {
    width: f64,
    height: f64,

    // Element counts
    rect_count: usize,
    circle_count: usize,
    ellipse_count: usize,
    path_count: usize,
    line_count: usize,
    polygon_count: usize,
    polyline_count: usize,
    text_count: usize,
    g_count: usize,

    // Colors used (color -> count)
    fills: HashMap<String, usize>,
    strokes: HashMap<String, usize>,

    // Text content
    text_contents: Vec<String>,

    // Font info
    fonts: HashSet<String>,
    font_sizes: HashSet<String>,
}

/// Difference found between two SVGs
#[derive(Debug)]
struct Difference {
    category: String,
    description: String,
    severity: Severity,
}

#[derive(Debug, Clone, Copy)]
enum Severity {
    Critical,
    Major,
    Minor,
    Info,
}

impl Severity {
    fn label(&self) -> &'static str {
        match self {
            Severity::Critical => "CRITICAL",
            Severity::Major => "MAJOR",
            Severity::Minor => "MINOR",
            Severity::Info => "INFO",
        }
    }

    fn color(&self) -> &'static str {
        match self {
            Severity::Critical => "#dc3545",
            Severity::Major => "#fd7e14",
            Severity::Minor => "#ffc107",
            Severity::Info => "#17a2b8",
        }
    }
}

/// Comparison result for a single diagram
#[derive(Debug)]
struct ComparisonResult {
    name: String,
    rs_metrics: SvgMetrics,
    ref_metrics: SvgMetrics,
    differences: Vec<Difference>,
    similarity_score: f64,
}

fn parse_dimension(value: &str) -> f64 {
    if value.is_empty() || value.contains('%') {
        return 0.0;
    }
    // Extract numeric part
    let numeric: String = value
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    numeric.parse().unwrap_or(0.0)
}

fn parse_viewbox(viewbox: &str) -> (f64, f64) {
    let parts: Vec<&str> = viewbox.split_whitespace().collect();
    if parts.len() >= 4 {
        let w = parts[2].parse().unwrap_or(0.0);
        let h = parts[3].parse().unwrap_or(0.0);
        (w, h)
    } else {
        (0.0, 0.0)
    }
}

fn extract_metrics(svg_content: &str) -> SvgMetrics {
    let mut metrics = SvgMetrics::default();

    // Parse with roxmltree
    let doc = match roxmltree::Document::parse(svg_content) {
        Ok(doc) => doc,
        Err(_) => return metrics,
    };

    let root = doc.root_element();

    // Get dimensions from root
    if let Some(viewbox) = root.attribute("viewBox") {
        let (w, h) = parse_viewbox(viewbox);
        metrics.width = w;
        metrics.height = h;
    }

    if metrics.width == 0.0 {
        if let Some(w) = root.attribute("width") {
            metrics.width = parse_dimension(w);
        }
    }
    if metrics.height == 0.0 {
        if let Some(h) = root.attribute("height") {
            metrics.height = parse_dimension(h);
        }
    }

    // Count elements and extract attributes
    for node in doc.descendants() {
        if !node.is_element() {
            continue;
        }

        let tag = node.tag_name().name();

        match tag {
            "rect" => metrics.rect_count += 1,
            "circle" => metrics.circle_count += 1,
            "ellipse" => metrics.ellipse_count += 1,
            "path" => metrics.path_count += 1,
            "line" => metrics.line_count += 1,
            "polygon" => metrics.polygon_count += 1,
            "polyline" => metrics.polyline_count += 1,
            "text" => {
                metrics.text_count += 1;
                // Extract text content - only get direct text or tspan text, not both
                // First try direct text node
                if let Some(direct_text) = node.text() {
                    let text = direct_text.trim().to_string();
                    if !text.is_empty() {
                        metrics.text_contents.push(text);
                    }
                } else {
                    // Otherwise collect from tspans only
                    let text: String = node
                        .children()
                        .filter(|n| n.is_element() && n.tag_name().name() == "tspan")
                        .filter_map(|n| n.text())
                        .map(|s| s.trim())
                        .collect::<Vec<_>>()
                        .join(" ");
                    if !text.is_empty() {
                        metrics.text_contents.push(text);
                    }
                }
            }
            "tspan" => {
                // Don't double-count tspan - handled by parent text element
            }
            "g" => metrics.g_count += 1,
            "foreignObject" => {
                // mermaid-js uses foreignObject for HTML text
                metrics.text_count += 1;
                for descendant in node.descendants() {
                    if let Some(text) = descendant.text() {
                        let text = text.trim().to_string();
                        if !text.is_empty() {
                            metrics.text_contents.push(text);
                        }
                    }
                }
            }
            _ => {}
        }

        // Extract colors
        if let Some(fill) = node.attribute("fill") {
            if fill != "none" && !fill.is_empty() {
                *metrics.fills.entry(fill.to_lowercase()).or_insert(0) += 1;
            }
        }
        if let Some(stroke) = node.attribute("stroke") {
            if stroke != "none" && !stroke.is_empty() {
                *metrics.strokes.entry(stroke.to_lowercase()).or_insert(0) += 1;
            }
        }

        // Extract font info
        if let Some(font) = node.attribute("font-family") {
            metrics.fonts.insert(font.to_string());
        }
        if let Some(size) = node.attribute("font-size") {
            metrics.font_sizes.insert(size.to_string());
        }
    }

    metrics
}

fn compare_metrics(name: &str, rs: &SvgMetrics, ref_metrics: &SvgMetrics) -> Vec<Difference> {
    let mut diffs = Vec::new();

    // Dimension differences
    let width_diff = (rs.width - ref_metrics.width).abs();
    let height_diff = (rs.height - ref_metrics.height).abs();

    if width_diff > 100.0 {
        diffs.push(Difference {
            category: "Dimensions".to_string(),
            description: format!(
                "Width: rs={:.0} vs ref={:.0} (diff={:.0})",
                rs.width, ref_metrics.width, width_diff
            ),
            severity: Severity::Major,
        });
    } else if width_diff > 50.0 {
        diffs.push(Difference {
            category: "Dimensions".to_string(),
            description: format!(
                "Width: rs={:.0} vs ref={:.0} (diff={:.0})",
                rs.width, ref_metrics.width, width_diff
            ),
            severity: Severity::Minor,
        });
    }

    if height_diff > 100.0 {
        diffs.push(Difference {
            category: "Dimensions".to_string(),
            description: format!(
                "Height: rs={:.0} vs ref={:.0} (diff={:.0})",
                rs.height, ref_metrics.height, height_diff
            ),
            severity: Severity::Major,
        });
    } else if height_diff > 50.0 {
        diffs.push(Difference {
            category: "Dimensions".to_string(),
            description: format!(
                "Height: rs={:.0} vs ref={:.0} (diff={:.0})",
                rs.height, ref_metrics.height, height_diff
            ),
            severity: Severity::Minor,
        });
    }

    // Element count differences
    let element_checks = [
        ("rect", rs.rect_count, ref_metrics.rect_count),
        ("circle", rs.circle_count, ref_metrics.circle_count),
        ("ellipse", rs.ellipse_count, ref_metrics.ellipse_count),
        ("path", rs.path_count, ref_metrics.path_count),
        ("line", rs.line_count, ref_metrics.line_count),
        ("polygon", rs.polygon_count, ref_metrics.polygon_count),
        ("polyline", rs.polyline_count, ref_metrics.polyline_count),
        ("text", rs.text_count, ref_metrics.text_count),
        ("g", rs.g_count, ref_metrics.g_count),
    ];

    for (elem_type, rs_count, ref_count) in element_checks {
        if rs_count != ref_count {
            let diff_pct = if ref_count > 0 {
                ((rs_count as f64 - ref_count as f64) / ref_count as f64 * 100.0).abs()
            } else if rs_count > 0 {
                100.0
            } else {
                0.0
            };

            let severity = if diff_pct > 50.0 || (rs_count == 0 && ref_count > 0) {
                Severity::Critical
            } else if diff_pct > 20.0 {
                Severity::Major
            } else {
                Severity::Minor
            };

            diffs.push(Difference {
                category: "Elements".to_string(),
                description: format!("{}: rs={} vs ref={}", elem_type, rs_count, ref_count),
                severity,
            });
        }
    }

    // Color differences
    let rs_fills: HashSet<_> = rs.fills.keys().cloned().collect();
    let ref_fills: HashSet<_> = ref_metrics.fills.keys().cloned().collect();

    let rs_only_fills: Vec<_> = rs_fills.difference(&ref_fills).take(5).cloned().collect();
    let ref_only_fills: Vec<_> = ref_fills.difference(&rs_fills).take(5).cloned().collect();

    if !ref_only_fills.is_empty() {
        diffs.push(Difference {
            category: "Colors".to_string(),
            description: format!("Fills only in ref: {}", ref_only_fills.join(", ")),
            severity: Severity::Minor,
        });
    }
    if !rs_only_fills.is_empty() {
        diffs.push(Difference {
            category: "Colors".to_string(),
            description: format!("Fills only in rs: {}", rs_only_fills.join(", ")),
            severity: Severity::Minor,
        });
    }

    // Text content differences
    let rs_texts: HashSet<_> = rs.text_contents.iter().cloned().collect();
    let ref_texts: HashSet<_> = ref_metrics.text_contents.iter().cloned().collect();

    let missing_texts: Vec<_> = ref_texts.difference(&rs_texts).take(5).cloned().collect();
    let extra_texts: Vec<_> = rs_texts.difference(&ref_texts).take(5).cloned().collect();

    if !missing_texts.is_empty() {
        diffs.push(Difference {
            category: "Text".to_string(),
            description: format!("Missing in rs: {:?}", missing_texts),
            severity: Severity::Major,
        });
    }
    if !extra_texts.is_empty() {
        diffs.push(Difference {
            category: "Text".to_string(),
            description: format!("Extra in rs: {:?}", extra_texts),
            severity: Severity::Minor,
        });
    }

    diffs
}

fn calculate_similarity(rs: &SvgMetrics, ref_metrics: &SvgMetrics) -> f64 {
    let mut score = 1.0;

    // Dimension penalty (max 20%)
    let dim_diff = ((rs.width - ref_metrics.width).abs() + (rs.height - ref_metrics.height).abs())
        / (ref_metrics.width + ref_metrics.height + 1.0);
    score -= (dim_diff * 0.2).min(0.2);

    // Element count penalty (max 40%)
    let total_ref_elems = ref_metrics.rect_count
        + ref_metrics.circle_count
        + ref_metrics.path_count
        + ref_metrics.text_count
        + ref_metrics.line_count;
    let total_rs_elems =
        rs.rect_count + rs.circle_count + rs.path_count + rs.text_count + rs.line_count;

    if total_ref_elems > 0 {
        let elem_diff =
            (total_rs_elems as f64 - total_ref_elems as f64).abs() / total_ref_elems as f64;
        score -= (elem_diff * 0.4).min(0.4);
    }

    // Text content penalty (max 30%)
    let rs_texts: HashSet<_> = rs.text_contents.iter().collect();
    let ref_texts: HashSet<_> = ref_metrics.text_contents.iter().collect();
    let common = rs_texts.intersection(&ref_texts).count();
    let total = rs_texts.len().max(ref_texts.len());
    if total > 0 {
        let text_similarity = common as f64 / total as f64;
        score -= (1.0 - text_similarity) * 0.3;
    }

    score.max(0.0)
}

fn generate_html_report(results: &[ComparisonResult], output_path: &Path) -> std::io::Result<()> {
    let mut html = String::from(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>SVG Comparison Report - mermaid-rs vs mermaid.js</title>
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
        .similarity-badge {
            padding: 5px 12px;
            border-radius: 20px;
            font-weight: bold;
            font-size: 0.9em;
        }
        .similarity-high { background: #d4edda; color: #155724; }
        .similarity-medium { background: #fff3cd; color: #856404; }
        .similarity-low { background: #f8d7da; color: #721c24; }
        .comparison-grid {
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 0;
        }
        .svg-panel {
            padding: 20px;
            text-align: center;
            border-right: 1px solid #eee;
            background: #fafafa;
        }
        .svg-panel:last-child { border-right: none; }
        .svg-panel h4 { margin: 0 0 10px 0; color: #666; }
        .svg-panel img, .svg-panel object {
            max-width: 100%;
            height: auto;
            background: white;
            border: 1px solid #ddd;
            border-radius: 4px;
        }
        .metrics-panel {
            padding: 15px 20px;
            background: #f8f9fa;
            border-top: 1px solid #eee;
        }
        .metrics-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
            gap: 10px;
            font-size: 0.85em;
        }
        .metric { color: #666; }
        .metric span { font-weight: bold; color: #333; }
        .differences-panel {
            padding: 15px 20px;
            border-top: 1px solid #eee;
        }
        .diff-list { list-style: none; padding: 0; margin: 0; }
        .diff-item {
            padding: 8px 12px;
            margin: 5px 0;
            border-radius: 4px;
            font-size: 0.9em;
            display: flex;
            align-items: center;
            gap: 10px;
        }
        .diff-badge {
            padding: 2px 8px;
            border-radius: 3px;
            font-size: 0.75em;
            font-weight: bold;
            color: white;
        }
        .diff-category {
            color: #666;
            min-width: 80px;
        }
        .no-diffs {
            color: #28a745;
            font-style: italic;
        }
    </style>
</head>
<body>
    <h1>SVG Comparison Report</h1>
    <p>mermaid-rs vs mermaid.js (reference)</p>
"#,
    );

    // Summary section
    let total = results.len();
    let high_similarity = results.iter().filter(|r| r.similarity_score >= 0.9).count();
    let has_critical = results
        .iter()
        .filter(|r| {
            r.differences
                .iter()
                .any(|d| matches!(d.severity, Severity::Critical))
        })
        .count();
    let avg_similarity: f64 =
        results.iter().map(|r| r.similarity_score).sum::<f64>() / total.max(1) as f64;

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
                <div class="stat-label">Avg Similarity</div>
            </div>
            <div class="stat-box">
                <div class="stat-value">{}</div>
                <div class="stat-label">High Match (>90%)</div>
            </div>
            <div class="stat-box">
                <div class="stat-value">{}</div>
                <div class="stat-label">Has Critical Issues</div>
            </div>
        </div>
    </div>
"#,
        total,
        avg_similarity * 100.0,
        high_similarity,
        has_critical
    ));

    // Individual diagram cards
    for result in results {
        let similarity_class = if result.similarity_score >= 0.9 {
            "similarity-high"
        } else if result.similarity_score >= 0.7 {
            "similarity-medium"
        } else {
            "similarity-low"
        };

        html.push_str(&format!(
            r#"
    <div class="diagram-card">
        <div class="diagram-header">
            <span class="diagram-name">{}</span>
            <span class="similarity-badge {}">{:.0}% match</span>
        </div>
        <div class="comparison-grid">
            <div class="svg-panel">
                <h4>mermaid.js (reference)</h4>
                <object type="image/svg+xml" data="{}_ref.svg" width="100%">Reference SVG</object>
            </div>
            <div class="svg-panel">
                <h4>mermaid-rs (ours)</h4>
                <object type="image/svg+xml" data="{}_rs.svg" width="100%">Our SVG</object>
            </div>
        </div>
        <div class="metrics-panel">
            <div class="metrics-grid">
                <div class="metric">Size (ref): <span>{:.0}x{:.0}</span></div>
                <div class="metric">Size (rs): <span>{:.0}x{:.0}</span></div>
                <div class="metric">Rects: <span>{}/{}</span></div>
                <div class="metric">Paths: <span>{}/{}</span></div>
                <div class="metric">Text: <span>{}/{}</span></div>
                <div class="metric">Circles: <span>{}/{}</span></div>
            </div>
        </div>
"#,
            result.name,
            similarity_class,
            result.similarity_score * 100.0,
            result.name,
            result.name,
            result.ref_metrics.width,
            result.ref_metrics.height,
            result.rs_metrics.width,
            result.rs_metrics.height,
            result.rs_metrics.rect_count,
            result.ref_metrics.rect_count,
            result.rs_metrics.path_count,
            result.ref_metrics.path_count,
            result.rs_metrics.text_count,
            result.ref_metrics.text_count,
            result.rs_metrics.circle_count,
            result.ref_metrics.circle_count,
        ));

        // Differences
        html.push_str(r#"        <div class="differences-panel">"#);
        if result.differences.is_empty() {
            html.push_str(r#"<p class="no-diffs">No significant differences detected</p>"#);
        } else {
            html.push_str(r#"<ul class="diff-list">"#);
            for diff in &result.differences {
                html.push_str(&format!(
                    r#"<li class="diff-item">
                        <span class="diff-badge" style="background:{}">{}</span>
                        <span class="diff-category">{}</span>
                        <span>{}</span>
                    </li>"#,
                    diff.severity.color(),
                    diff.severity.label(),
                    diff.category,
                    diff.description
                ));
            }
            html.push_str("</ul>");
        }
        html.push_str("</div></div>");
    }

    html.push_str("</body></html>");

    fs::write(output_path, html)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = Path::new("tools/gallery/output");

    let separator = "=".repeat(70);
    println!("{}", separator);
    println!("SVG COMPARISON: mermaid-rs vs mermaid.js");
    println!("{}", separator);
    println!();

    let diagrams = [
        "flowchart",
        "pie",
        "sequence",
        "class",
        "state",
        "er",
        "gantt",
    ];

    let mut results = Vec::new();

    for name in &diagrams {
        let rs_path = output_dir.join(format!("{}_rs.svg", name));
        let ref_path = output_dir.join(format!("{}_ref.svg", name));

        if !rs_path.exists() {
            println!("{}: mermaid-rs output missing!", name);
            continue;
        }
        if !ref_path.exists() {
            println!("{}: reference output missing!", name);
            continue;
        }

        let rs_svg = fs::read_to_string(&rs_path)?;
        let ref_svg = fs::read_to_string(&ref_path)?;

        let rs_metrics = extract_metrics(&rs_svg);
        let ref_metrics = extract_metrics(&ref_svg);

        let differences = compare_metrics(name, &rs_metrics, &ref_metrics);
        let similarity = calculate_similarity(&rs_metrics, &ref_metrics);

        println!("\n{}", name);
        println!("{}", "-".repeat(40));

        if differences.is_empty() {
            println!("  No significant differences detected");
        } else {
            for diff in &differences {
                println!(
                    "  [{}] {}: {}",
                    diff.severity.label(),
                    diff.category,
                    diff.description
                );
            }
        }

        println!(
            "  Size: {:.0}x{:.0} (ref) vs {:.0}x{:.0} (rs)",
            ref_metrics.width, ref_metrics.height, rs_metrics.width, rs_metrics.height
        );
        println!("  Similarity: {:.0}%", similarity * 100.0);

        results.push(ComparisonResult {
            name: name.to_string(),
            rs_metrics,
            ref_metrics,
            differences,
            similarity_score: similarity,
        });
    }

    // Summary
    println!("\n{}", separator);
    println!("SUMMARY");
    println!("{}", separator);

    let total = results.len();
    let with_diffs = results.iter().filter(|r| !r.differences.is_empty()).count();
    let avg_sim: f64 =
        results.iter().map(|r| r.similarity_score).sum::<f64>() / total.max(1) as f64;

    println!("\nDiagrams with differences: {}/{}", with_diffs, total);
    println!("Average similarity: {:.0}%", avg_sim * 100.0);

    for result in &results {
        let critical_count = result
            .differences
            .iter()
            .filter(|d| matches!(d.severity, Severity::Critical))
            .count();
        let major_count = result
            .differences
            .iter()
            .filter(|d| matches!(d.severity, Severity::Major))
            .count();

        if critical_count > 0 || major_count > 0 {
            println!(
                "  - {}: {} critical, {} major issues",
                result.name, critical_count, major_count
            );
        }
    }

    // Generate HTML report
    let report_path = output_dir.join("comparison_report.html");
    generate_html_report(&results, &report_path)?;
    println!("\nHTML report generated: {}", report_path.display());

    Ok(())
}
