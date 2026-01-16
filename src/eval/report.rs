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
pub fn text_summary(result: &EvalResult, output_dir: Option<&Path>) -> String {
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
        output.push_str(&format!(
            "  {:<12} {:>12} {:>12}\n",
            "Type", "Structural", "Visual"
        ));
        output.push_str(&format!("  {}\n", "-".repeat(38)));

        let mut types: Vec<(&String, &TypeStats)> = result.by_type.iter().collect();
        types.sort_by_key(|(name, _)| name.as_str());

        for (dtype, stats) in types {
            let structural_str = if stats.avg_structural > 0.0 {
                format!("{:>11.0}%", stats.avg_structural * 100.0)
            } else {
                "         -".to_string()
            };
            let visual_str = if stats.avg_ssim > 0.0 {
                format!("{:>11.0}%", stats.avg_ssim * 100.0)
            } else {
                "         -".to_string()
            };
            output.push_str(&format!(
                "  {:<12} {} {}\n",
                dtype, structural_str, visual_str
            ));

            if let Some(base_dir) = output_dir {
                let type_dir = base_dir.join(dtype);
                output.push_str(&format!("    SVG Location: {}\n", type_dir.display()));

                // List comparison PNGs for this type
                let mut pngs: Vec<String> = result
                    .diagrams
                    .iter()
                    .filter(|d| &d.diagram_type == dtype)
                    .map(|d| {
                        let safe_name = d.name.replace(['/', ' '], "_");
                        format!("{}_comparison.png", safe_name)
                    })
                    .filter(|name| type_dir.join(name).exists())
                    .collect();

                // Sort for consistent output
                pngs.sort();

                if !pngs.is_empty() {
                    output.push_str("    Comparison PNGs:\n");
                    for png in pngs {
                        output.push_str(&format!("      {}\n", type_dir.join(png).display()));
                    }
                }
                output.push('\n');
            }
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

/// Generate a detailed text report with issues
pub fn text_detailed(result: &EvalResult, output_dir: Option<&Path>) -> String {
    let mut output = text_summary(result, output_dir);

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

            if let Some(base_dir) = output_dir {
                let safe_name = diagram.name.replace(['/', ' '], "_");
                let type_dir = base_dir.join(&diagram.diagram_type);

                if diagram.selkie_svg.is_some() {
                    output.push_str(&format!(
                        "  SVG: {}\n",
                        type_dir.join(format!("{}_selkie.svg", safe_name)).display()
                    ));
                }
                if diagram.reference_svg.is_some() {
                    output.push_str(&format!(
                        "  Ref: {}\n",
                        type_dir
                            .join(format!("{}_reference.svg", safe_name))
                            .display()
                    ));
                }

                let png_path = type_dir.join(format!("{}_comparison.png", safe_name));
                if png_path.exists() {
                    output.push_str(&format!("  PNG: {}\n", png_path.display()));
                }
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

/// Get reference implementation paths for a diagram type
fn get_reference_paths(diagram_type: &str) -> ReferencePathInfo {
    let mermaid_base = "reference-implementations/mermaid/packages/mermaid/src/diagrams";

    match diagram_type {
        "flowchart" => ReferencePathInfo {
            parser: format!("{}/flowchart/parser/flowParser.ts", mermaid_base),
            renderer: format!("{}/flowchart/flowRenderer-v2.ts", mermaid_base),
            db: format!("{}/flowchart/flowDb.ts", mermaid_base),
            styles: Some(format!("{}/flowchart/styles.ts", mermaid_base)),
            layout_hint: Some(
                "Uses dagre for layout: reference-implementations/dagre/".to_string(),
            ),
        },
        "sequence" => ReferencePathInfo {
            parser: format!("{}/sequence/sequenceParser.ts", mermaid_base),
            renderer: format!("{}/sequence/sequenceRenderer.ts", mermaid_base),
            db: format!("{}/sequence/sequenceDb.ts", mermaid_base),
            styles: Some(format!("{}/sequence/styles.ts", mermaid_base)),
            layout_hint: None,
        },
        "class" => ReferencePathInfo {
            parser: format!("{}/class/classParser.ts", mermaid_base),
            renderer: format!("{}/class/classRenderer-v2.ts", mermaid_base),
            db: format!("{}/class/classDb.ts", mermaid_base),
            styles: Some(format!("{}/class/styles.ts", mermaid_base)),
            layout_hint: Some("Uses dagre or elk for layout".to_string()),
        },
        "state" => ReferencePathInfo {
            parser: format!("{}/state/stateParser.ts", mermaid_base),
            renderer: format!("{}/state/stateRenderer-v3-unified.ts", mermaid_base),
            db: format!("{}/state/stateDb.ts", mermaid_base),
            styles: Some(format!("{}/state/styles.ts", mermaid_base)),
            layout_hint: None,
        },
        "er" => ReferencePathInfo {
            parser: format!("{}/er/erParser.ts", mermaid_base),
            renderer: format!("{}/er/erRenderer.ts", mermaid_base),
            db: format!("{}/er/erDb.ts", mermaid_base),
            styles: Some(format!("{}/er/styles.ts", mermaid_base)),
            layout_hint: None,
        },
        "gantt" => ReferencePathInfo {
            parser: format!("{}/gantt/ganttParser.ts", mermaid_base),
            renderer: format!("{}/gantt/ganttRenderer.ts", mermaid_base),
            db: format!("{}/gantt/ganttDb.ts", mermaid_base),
            styles: Some(format!("{}/gantt/styles.ts", mermaid_base)),
            layout_hint: None,
        },
        "pie" => ReferencePathInfo {
            parser: format!("{}/pie/pieParser.ts", mermaid_base),
            renderer: format!("{}/pie/pieRenderer.ts", mermaid_base),
            db: format!("{}/pie/pieDb.ts", mermaid_base),
            styles: Some(format!("{}/pie/styles.ts", mermaid_base)),
            layout_hint: None,
        },
        "architecture" => ReferencePathInfo {
            parser: format!("{}/architecture/architectureParser.ts", mermaid_base),
            renderer: format!("{}/architecture/architectureRenderer.ts", mermaid_base),
            db: format!("{}/architecture/architectureDb.ts", mermaid_base),
            styles: Some(format!(
                "{}/architecture/architectureStyles.ts",
                mermaid_base
            )),
            layout_hint: None,
        },
        _ => ReferencePathInfo {
            parser: format!("{}/{}/parser.ts", mermaid_base, diagram_type),
            renderer: format!("{}/{}/renderer.ts", mermaid_base, diagram_type),
            db: format!("{}/{}/db.ts", mermaid_base, diagram_type),
            styles: None,
            layout_hint: None,
        },
    }
}

/// Get selkie implementation paths for a diagram type
fn get_selkie_paths(diagram_type: &str) -> SelkiePathInfo {
    match diagram_type {
        "flowchart" => SelkiePathInfo {
            parser: "src/parser/flowchart.rs".to_string(),
            renderer: "src/render/flowchart.rs".to_string(),
            types: Some("src/types/flowchart.rs".to_string()),
        },
        "sequence" => SelkiePathInfo {
            parser: "src/parser/sequence.rs".to_string(),
            renderer: "src/render/sequence.rs".to_string(),
            types: Some("src/types/sequence.rs".to_string()),
        },
        "class" => SelkiePathInfo {
            parser: "src/parser/class.rs".to_string(),
            renderer: "src/render/class.rs".to_string(),
            types: Some("src/types/class.rs".to_string()),
        },
        "state" => SelkiePathInfo {
            parser: "src/parser/state.rs".to_string(),
            renderer: "src/render/state.rs".to_string(),
            types: Some("src/types/state.rs".to_string()),
        },
        "er" => SelkiePathInfo {
            parser: "src/parser/er.rs".to_string(),
            renderer: "src/render/er.rs".to_string(),
            types: Some("src/types/er.rs".to_string()),
        },
        "gantt" => SelkiePathInfo {
            parser: "src/parser/gantt.rs".to_string(),
            renderer: "src/render/gantt.rs".to_string(),
            types: Some("src/types/gantt.rs".to_string()),
        },
        "pie" => SelkiePathInfo {
            parser: "src/parser/pie.rs".to_string(),
            renderer: "src/render/pie.rs".to_string(),
            types: Some("src/types/pie.rs".to_string()),
        },
        "architecture" => SelkiePathInfo {
            parser: "src/parser/architecture.rs".to_string(),
            renderer: "src/render/architecture.rs".to_string(),
            types: Some("src/types/architecture.rs".to_string()),
        },
        _ => SelkiePathInfo {
            parser: format!("src/parser/{}.rs", diagram_type),
            renderer: format!("src/render/{}.rs", diagram_type),
            types: Some(format!("src/types/{}.rs", diagram_type)),
        },
    }
}

/// Reference implementation path information
struct ReferencePathInfo {
    parser: String,
    renderer: String,
    db: String,
    styles: Option<String>,
    layout_hint: Option<String>,
}

/// Selkie implementation path information
struct SelkiePathInfo {
    parser: String,
    renderer: String,
    types: Option<String>,
}

/// Generate an AI-agent friendly report with structured per-diagram output
pub fn text_agent_friendly(result: &EvalResult, output_dir: Option<&Path>) -> String {
    let mut output = String::new();

    // Header with clear machine-parseable format
    output.push_str(
        "================================================================================\n",
    );
    output.push_str("SELKIE EVALUATION REPORT (AI-AGENT FORMAT)\n");
    output.push_str(
        "================================================================================\n\n",
    );

    // Report file locations (important for AI agents to know where to read more)
    if let Some(dir) = output_dir {
        output.push_str("## REPORT FILES\n\n");
        output.push_str(&format!(
            "- **Summary JSON**: {}\n",
            dir.join("report.json").display()
        ));
        output.push_str(&format!(
            "- **HTML Report**: {}\n",
            dir.join("index.html").display()
        ));
        output.push_str(&format!(
            "- **Per-diagram files**: {}/{{type}}/{{name}}.json, *_selkie.svg, *_reference.svg\n",
            dir.display()
        ));
        output.push('\n');
    }

    // Quick summary for prioritization
    output.push_str("## SUMMARY\n\n");
    output.push_str(&format!("- Total diagrams: {}\n", result.total));
    output.push_str(&format!(
        "- Passing: {} ({:.1}%)\n",
        result.matching, result.parity_percent
    ));
    output.push_str(&format!("- Errors: {}\n", result.issue_counts.errors));
    output.push_str(&format!("- Warnings: {}\n", result.issue_counts.warnings));
    output.push('\n');

    // Group diagrams by status for easy prioritization
    let errors: Vec<_> = result
        .diagrams
        .iter()
        .filter(|d| d.status == Status::Error)
        .collect();
    let warnings: Vec<_> = result
        .diagrams
        .iter()
        .filter(|d| d.status == Status::Warning)
        .collect();
    let matches: Vec<_> = result
        .diagrams
        .iter()
        .filter(|d| d.status == Status::Match)
        .collect();

    // Priority order for fixing
    if !errors.is_empty() {
        output.push_str("## PRIORITY: FIX THESE FIRST (ERRORS)\n\n");
        for (i, diagram) in errors.iter().enumerate() {
            output.push_str(&format_diagram_for_agent(diagram, i + 1, output_dir));
        }
    }

    if !warnings.is_empty() {
        output.push_str("## WARNINGS (Lower Priority)\n\n");
        for (i, diagram) in warnings.iter().enumerate() {
            output.push_str(&format_diagram_for_agent(diagram, i + 1, output_dir));
        }
    }

    // Show passing diagrams briefly
    if !matches.is_empty() {
        output.push_str(&format!("## PASSING ({} diagrams)\n\n", matches.len()));
        for diagram in matches.iter() {
            output.push_str(&format!(
                "- [OK] {} ({})\n",
                diagram.name, diagram.diagram_type
            ));
        }
        output.push('\n');
    }

    // Investigation guide at the end
    output.push_str(
        "================================================================================\n",
    );
    output.push_str("## INVESTIGATION GUIDE FOR AI AGENTS\n");
    output.push_str(
        "================================================================================\n\n",
    );

    output.push_str("### Setting Up Reference Implementations\n\n");
    output.push_str("If reference implementations are not initialized, run:\n");
    output.push_str("```bash\n");
    output.push_str("git submodule init\n");
    output.push_str("git submodule update --depth 1\n");
    output.push_str("```\n\n");

    output.push_str("### Issue Types and What They Mean\n\n");
    output.push_str("- **node_count**: Mismatch in number of shapes/boxes rendered\n");
    output.push_str(
        "  → Check: Parser may not be extracting all nodes, or renderer skipping them\n\n",
    );
    output.push_str("- **edge_count**: Mismatch in number of connecting lines/arrows\n");
    output
        .push_str("  → Check: Parser may miss edge definitions, or renderer not drawing them\n\n");
    output.push_str("- **labels_missing**: Text labels present in reference but not in selkie\n");
    output.push_str("  → Check: Parser may not extract label text, or renderer not placing it\n\n");
    output.push_str("- **dimensions**: SVG size significantly different from reference\n");
    output.push_str("  → Check: Layout algorithm, padding, or font metrics differ\n\n");
    output.push_str("- **shapes**: Different number of specific SVG elements (rect, path, etc.)\n");
    output.push_str("  → Check: Renderer using different primitives than reference\n\n");
    output.push_str("- **z_order**: Text rendered before shapes (may be hidden behind them)\n");
    output.push_str("  → SVG renders elements in document order - later elements appear on top\n");
    output.push_str("  → Fix: In renderer, emit shapes BEFORE text labels within each group\n");
    output.push_str("  → Check: Compare element order in selkie vs reference SVG files\n\n");

    output.push_str("### Debugging Workflow\n\n");
    output.push_str("1. Read the diagram source to understand what should be rendered\n");
    output.push_str("2. Compare the selkie SVG vs reference SVG (paths shown per-diagram above)\n");
    output.push_str(
        "3. For parsing issues: Check the parser file and compare with reference parser\n",
    );
    output.push_str("4. For rendering issues: Check the renderer and compare output structure\n");
    output.push_str("5. Use `cargo test -p selkie -- <diagram_type>` to run related tests\n\n");

    output
}

/// Format a single diagram for agent-friendly output
fn format_diagram_for_agent(
    diagram: &super::DiagramResult,
    index: usize,
    output_dir: Option<&Path>,
) -> String {
    let mut output = String::new();

    output.push_str(
        "--------------------------------------------------------------------------------\n",
    );
    output.push_str(&format!("### Diagram {}: {}\n\n", index, diagram.name));

    output.push_str(&format!("**Type:** {}\n", diagram.diagram_type));
    output.push_str(&format!("**Status:** {:?}\n", diagram.status));

    if let Some(ssim) = diagram.visual_similarity {
        output.push_str(&format!(
            "**Visual Similarity (SSIM):** {:.1}%\n",
            ssim * 100.0
        ));
    }
    if let Some(structural) = diagram.structural_similarity {
        output.push_str(&format!(
            "**Structural Similarity:** {:.1}%\n",
            structural * 100.0
        ));
    }
    output.push('\n');

    // File locations for comparison
    if let Some(base_dir) = output_dir {
        let safe_name = diagram.name.replace(['/', ' '], "_");
        let type_dir = base_dir.join(&diagram.diagram_type);

        output.push_str("**Files:**\n");
        output.push_str(&format!(
            "- Full details: {}\n",
            type_dir
                .join(format!("{}_comparison.json", safe_name))
                .display()
        ));
        if diagram.selkie_svg.is_some() {
            output.push_str(&format!(
                "- Selkie SVG: {}\n",
                type_dir.join(format!("{}_selkie.svg", safe_name)).display()
            ));
        }
        if diagram.reference_svg.is_some() {
            output.push_str(&format!(
                "- Reference SVG: {}\n",
                type_dir
                    .join(format!("{}_reference.svg", safe_name))
                    .display()
            ));
        }
        let png_path = type_dir.join(format!("{}_comparison.png", safe_name));
        if png_path.exists() {
            output.push_str(&format!("- Comparison PNG: {}\n", png_path.display()));
        }
        output.push('\n');
    }

    // Issues with clear actionable format
    if !diagram.issues.is_empty() {
        output.push_str("**Issues Found:**\n\n");
        for issue in &diagram.issues {
            let level_str = match issue.level {
                Level::Error => "ERROR",
                Level::Warning => "WARN",
                Level::Info => "INFO",
            };
            output.push_str(&format!(
                "- [{}] **{}**: {}\n",
                level_str, issue.check, issue.message
            ));
            if let (Some(expected), Some(actual)) = (&issue.expected, &issue.actual) {
                output.push_str(&format!("  - Expected: {}\n", expected));
                output.push_str(&format!("  - Actual: {}\n", actual));
            }
        }
        output.push('\n');
    }

    // Source code for context
    if let Some(ref source) = diagram.diagram_text {
        output.push_str("**Diagram Source:**\n");
        output.push_str("```mermaid\n");
        // Truncate very long sources
        if source.len() > 1000 {
            output.push_str(&source[..1000]);
            output.push_str("\n... (truncated)\n");
        } else {
            output.push_str(source);
            output.push('\n');
        }
        output.push_str("```\n\n");
    }

    // Reference implementation pointers
    let ref_paths = get_reference_paths(&diagram.diagram_type);
    let selkie_paths = get_selkie_paths(&diagram.diagram_type);

    output.push_str("**Where to Look (Selkie Implementation):**\n");
    output.push_str(&format!("- Parser: {}\n", selkie_paths.parser));
    output.push_str(&format!("- Renderer: {}\n", selkie_paths.renderer));
    if let Some(types) = &selkie_paths.types {
        output.push_str(&format!("- Types: {}\n", types));
    }
    output.push('\n');

    output.push_str("**Reference Implementation (mermaid.js):**\n");
    output.push_str(&format!("- Parser: {}\n", ref_paths.parser));
    output.push_str(&format!("- Renderer: {}\n", ref_paths.renderer));
    output.push_str(&format!("- Database/State: {}\n", ref_paths.db));
    if let Some(styles) = &ref_paths.styles {
        output.push_str(&format!("- Styles: {}\n", styles));
    }
    if let Some(hint) = &ref_paths.layout_hint {
        output.push_str(&format!("- Note: {}\n", hint));
    }
    output.push('\n');

    output
}

/// Write JSON report to file
pub fn write_json(result: &EvalResult, path: &Path) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(result).map_err(std::io::Error::other)?;
    fs::write(path, json)
}

/// Summary report structure (without full SVG/diagram data)
#[derive(Debug, serde::Serialize)]
struct SummaryReport {
    total: usize,
    matching: usize,
    parity_percent: f64,
    avg_visual_similarity: f64,
    by_type: std::collections::HashMap<String, super::TypeStats>,
    issue_counts: super::IssueCounts,
    /// Index of all diagram JSON files
    diagrams: Vec<DiagramIndexEntry>,
}

/// Index entry for a diagram (points to its JSON file)
#[derive(Debug, serde::Serialize)]
struct DiagramIndexEntry {
    name: String,
    diagram_type: String,
    status: super::Status,
    json_file: String,
    error_count: usize,
    warning_count: usize,
}

/// Write JSON reports: one per diagram example in type directories
///
/// Creates:
/// - `report.json` - Summary with overall stats and index of diagram files
/// - `{type}/{name}.json` - Full results for each diagram example
pub fn write_json_by_type(result: &EvalResult, output_dir: &Path) -> std::io::Result<()> {
    let mut diagram_index: Vec<DiagramIndexEntry> = Vec::new();

    // Write per-diagram JSON files in type directories
    for diagram in &result.diagrams {
        let type_dir = output_dir.join(&diagram.diagram_type);
        fs::create_dir_all(&type_dir)?;

        let safe_name = diagram.name.replace(['/', ' '], "_");
        let filename = format!("{}_comparison.json", safe_name);
        let path = type_dir.join(&filename);

        // Count issues for index
        let mut errors = 0;
        let mut warnings = 0;
        for issue in &diagram.issues {
            match issue.level {
                super::Level::Error => errors += 1,
                super::Level::Warning => warnings += 1,
                super::Level::Info => {}
            }
        }

        // Write the diagram JSON
        let json = serde_json::to_string_pretty(&diagram).map_err(std::io::Error::other)?;
        fs::write(&path, json)?;

        // Add to index
        diagram_index.push(DiagramIndexEntry {
            name: diagram.name.clone(),
            diagram_type: diagram.diagram_type.clone(),
            status: diagram.status,
            json_file: format!("{}/{}", diagram.diagram_type, filename),
            error_count: errors,
            warning_count: warnings,
        });
    }

    // Sort index by type then name for consistent output
    diagram_index.sort_by(|a, b| {
        a.diagram_type
            .cmp(&b.diagram_type)
            .then_with(|| a.name.cmp(&b.name))
    });

    // Write summary report.json (with index but without full diagram data)
    let summary = SummaryReport {
        total: result.total,
        matching: result.matching,
        parity_percent: result.parity_percent,
        avg_visual_similarity: result.avg_visual_similarity,
        by_type: result.by_type.clone(),
        issue_counts: result.issue_counts.clone(),
        diagrams: diagram_index,
    };

    let summary_path = output_dir.join("report.json");
    let json = serde_json::to_string_pretty(&summary).map_err(std::io::Error::other)?;
    fs::write(&summary_path, json)?;

    Ok(())
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
    use crate::eval::{DiagramResult, Issue, IssueCounts, ParseResult, Status};
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
                        avg_structural: 0.85,
                    },
                ),
                (
                    "pie".to_string(),
                    TypeStats {
                        total: 5,
                        matching: 4,
                        parity_percent: 80.0,
                        avg_ssim: 0.91,
                        avg_structural: 0.90,
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

    fn make_test_result_with_errors() -> EvalResult {
        EvalResult {
            total: 3,
            matching: 1,
            parity_percent: 33.3,
            avg_visual_similarity: 0.75,
            by_type: HashMap::from([(
                "flowchart".to_string(),
                TypeStats {
                    total: 3,
                    matching: 1,
                    parity_percent: 33.3,
                    avg_ssim: 0.75,
                    avg_structural: 0.60,
                },
            )]),
            issue_counts: IssueCounts {
                errors: 2,
                warnings: 1,
                info: 0,
                visual_only: 0,
            },
            diagrams: vec![
                DiagramResult {
                    name: "test_error_diagram".to_string(),
                    source: Some("docs/sources/test.mmd".to_string()),
                    diagram_type: "flowchart".to_string(),
                    diagram_text: Some("flowchart LR\n    A --> B\n    B --> C".to_string()),
                    status: Status::Error,
                    visual_similarity: Some(0.65),
                    structural_similarity: Some(0.50),
                    structural_match: false,
                    issues: vec![
                        Issue::error("node_count", "Node count mismatch: expected 3, got 2")
                            .with_values("3", "2"),
                        Issue::error("labels_missing", "Missing labels: [\"C\"]")
                            .with_values("[\"A\", \"B\", \"C\"]", "[\"A\", \"B\"]"),
                    ],
                    parse_result: ParseResult {
                        selkie_success: true,
                        selkie_error: None,
                        reference_success: true,
                        reference_error: None,
                    },
                    render_result: None,
                    selkie_svg: Some("<svg></svg>".to_string()),
                    reference_svg: Some("<svg></svg>".to_string()),
                },
                DiagramResult {
                    name: "test_warning_diagram".to_string(),
                    source: None,
                    diagram_type: "flowchart".to_string(),
                    diagram_text: Some("flowchart TD\n    X --> Y".to_string()),
                    status: Status::Warning,
                    visual_similarity: Some(0.80),
                    structural_similarity: Some(0.85),
                    structural_match: true,
                    issues: vec![Issue::warning(
                        "dimensions",
                        "Width differs by 25%: expected 400, got 500",
                    )
                    .with_values("400", "500")],
                    parse_result: ParseResult {
                        selkie_success: true,
                        selkie_error: None,
                        reference_success: true,
                        reference_error: None,
                    },
                    render_result: None,
                    selkie_svg: Some("<svg></svg>".to_string()),
                    reference_svg: Some("<svg></svg>".to_string()),
                },
                DiagramResult {
                    name: "test_passing_diagram".to_string(),
                    source: None,
                    diagram_type: "flowchart".to_string(),
                    diagram_text: Some("flowchart LR\n    P --> Q".to_string()),
                    status: Status::Match,
                    visual_similarity: Some(0.95),
                    structural_similarity: Some(1.0),
                    structural_match: true,
                    issues: vec![],
                    parse_result: ParseResult {
                        selkie_success: true,
                        selkie_error: None,
                        reference_success: true,
                        reference_error: None,
                    },
                    render_result: None,
                    selkie_svg: Some("<svg></svg>".to_string()),
                    reference_svg: Some("<svg></svg>".to_string()),
                },
            ],
        }
    }

    #[test]
    fn test_text_summary() {
        let result = make_test_result();
        let summary = text_summary(&result, None);
        assert!(summary.contains("80.0%"));
        assert!(summary.contains("8/10"));
        assert!(summary.contains("flowchart"));
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a & b"), "a &amp; b");
    }

    #[test]
    fn test_agent_friendly_output_with_errors() {
        let result = make_test_result_with_errors();
        let output = text_agent_friendly(&result, None);

        // Check header
        assert!(output.contains("SELKIE EVALUATION REPORT (AI-AGENT FORMAT)"));

        // Check summary
        assert!(output.contains("Total diagrams: 3"));
        assert!(output.contains("Passing: 1"));
        assert!(output.contains("Errors: 2"));

        // Check priority section exists
        assert!(output.contains("PRIORITY: FIX THESE FIRST (ERRORS)"));

        // Check error diagram details
        assert!(output.contains("test_error_diagram"));
        assert!(output.contains("node_count"));
        assert!(output.contains("labels_missing"));
        assert!(output.contains("Expected: 3"));
        assert!(output.contains("Actual: 2"));

        // Check warning section exists
        assert!(output.contains("WARNINGS (Lower Priority)"));
        assert!(output.contains("test_warning_diagram"));

        // Check passing section
        assert!(output.contains("PASSING"));
        assert!(output.contains("test_passing_diagram"));

        // Check investigation guide
        assert!(output.contains("INVESTIGATION GUIDE FOR AI AGENTS"));
        assert!(output.contains("git submodule"));

        // Check reference implementation paths are included
        assert!(output.contains("Where to Look (Selkie Implementation)"));
        assert!(output.contains("Reference Implementation (mermaid.js)"));
        assert!(output.contains("src/parser/flowchart.rs"));
        assert!(output.contains("flowchart/flowRenderer"));
    }

    #[test]
    fn test_reference_paths() {
        let flowchart_paths = get_reference_paths("flowchart");
        assert!(flowchart_paths.parser.contains("flowParser"));
        assert!(flowchart_paths.renderer.contains("flowRenderer"));

        let sequence_paths = get_reference_paths("sequence");
        assert!(sequence_paths.parser.contains("sequenceParser"));

        let pie_paths = get_reference_paths("pie");
        assert!(pie_paths.parser.contains("pieParser"));
    }

    #[test]
    fn test_selkie_paths() {
        let flowchart_paths = get_selkie_paths("flowchart");
        assert_eq!(flowchart_paths.parser, "src/parser/flowchart.rs");
        assert_eq!(flowchart_paths.renderer, "src/render/flowchart.rs");

        let sequence_paths = get_selkie_paths("sequence");
        assert_eq!(sequence_paths.parser, "src/parser/sequence.rs");
    }
}
