//! Cross-validate Rust renderer against the reference TypeScript implementation.
//!
//! This tool:
//! 1. Takes mermaid diagrams as input
//! 2. Renders each diagram with both Rust and mermaid.js implementations
//! 3. Compares the SVG structure and reports differences

use mermaid::render::svg::{CompareConfig, SvgStructure};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Serialize, Deserialize)]
struct RenderResult {
    success: bool,
    svg: Option<String>,
    structure: Option<SvgStructure>,
    error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ValidationEntry {
    diagram: String,
    name: String,
    rust_result: RenderResult,
    ts_result: RenderResult,
    comparison: Option<ComparisonSummary>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ComparisonSummary {
    matches: bool,
    similarity: f64,
    critical_differences: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct ValidationReport {
    total: usize,
    rust_rendered: usize,
    ts_rendered: usize,
    both_rendered: usize,
    matches: usize,
    entries: Vec<ValidationEntry>,
}

/// Render diagram with Rust implementation
fn render_with_rust(diagram: &str) -> RenderResult {
    match mermaid::parse(diagram) {
        Ok(parsed) => match mermaid::render(&parsed) {
            Ok(svg) => {
                let structure = SvgStructure::from_svg(&svg).ok();
                RenderResult {
                    success: true,
                    svg: Some(svg),
                    structure,
                    error: None,
                }
            }
            Err(e) => RenderResult {
                success: false,
                svg: None,
                structure: None,
                error: Some(e.to_string()),
            },
        },
        Err(e) => RenderResult {
            success: false,
            svg: None,
            structure: None,
            error: Some(format!("Parse error: {}", e)),
        },
    }
}

/// Render diagram with TypeScript mermaid.js
fn render_with_typescript(diagram: &str, validator_path: &Path) -> RenderResult {
    let script = validator_path.join("render_mermaid.mjs");

    let mut child = match Command::new("node")
        .arg(&script)
        .arg("--analyze")
        .arg("-")
        .current_dir(validator_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return RenderResult {
                success: false,
                svg: None,
                structure: None,
                error: Some(format!("Failed to spawn node: {}", e)),
            }
        }
    };

    // Write diagram to stdin
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(diagram.as_bytes());
    }

    let output = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => {
            return RenderResult {
                success: false,
                svg: None,
                structure: None,
                error: Some(format!("Failed to get output: {}", e)),
            }
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return RenderResult {
            success: false,
            svg: None,
            structure: None,
            error: Some(stderr.to_string()),
        };
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    #[derive(Deserialize)]
    struct TsOutput {
        success: bool,
        svg: Option<String>,
        structure: Option<TsStructure>,
        error: Option<String>,
    }

    #[derive(Deserialize)]
    struct TsStructure {
        width: f64,
        height: f64,
        #[serde(rename = "nodeCount")]
        node_count: usize,
        #[serde(rename = "edgeCount")]
        edge_count: usize,
        labels: Vec<String>,
        #[serde(rename = "markerCount")]
        marker_count: usize,
        #[serde(rename = "hasDefs")]
        has_defs: bool,
        #[serde(rename = "hasStyle")]
        has_style: bool,
    }

    match serde_json::from_str::<TsOutput>(&stdout) {
        Ok(result) => {
            if result.success {
                let structure = result.structure.map(|s| SvgStructure {
                    width: s.width,
                    height: s.height,
                    node_count: s.node_count,
                    edge_count: s.edge_count,
                    labels: s.labels,
                    shapes: Default::default(), // TS doesn't provide this in same format
                    marker_count: s.marker_count,
                    has_defs: s.has_defs,
                    has_style: s.has_style,
                });
                RenderResult {
                    success: true,
                    svg: result.svg,
                    structure,
                    error: None,
                }
            } else {
                RenderResult {
                    success: false,
                    svg: None,
                    structure: None,
                    error: result.error,
                }
            }
        }
        Err(e) => RenderResult {
            success: false,
            svg: None,
            structure: None,
            error: Some(format!("Invalid JSON: {} - {}", e, stdout)),
        },
    }
}

fn validate_diagram(diagram: &str, name: &str, validator_path: &Path) -> ValidationEntry {
    let rust_result = render_with_rust(diagram);
    let ts_result = render_with_typescript(diagram, validator_path);

    let comparison = if let (Some(rust_struct), Some(ts_struct)) =
        (&rust_result.structure, &ts_result.structure)
    {
        let config = CompareConfig::default();
        let result = rust_struct.compare(ts_struct, &config);
        Some(ComparisonSummary {
            matches: result.matches,
            similarity: result.similarity,
            critical_differences: result
                .differences
                .iter()
                .filter(|d| d.severity == mermaid::render::svg::structure::Severity::Critical)
                .map(|d| format!("{}: expected {}, got {}", d.field, d.expected, d.actual))
                .collect(),
        })
    } else {
        None
    };

    ValidationEntry {
        diagram: if diagram.len() > 200 {
            format!("{}...", &diagram[..200])
        } else {
            diagram.to_string()
        },
        name: name.to_string(),
        rust_result,
        ts_result,
        comparison,
    }
}

fn run_validation(
    diagrams: &[(String, String)], // (name, diagram)
    validator_path: &Path,
) -> ValidationReport {
    let mut report = ValidationReport {
        total: diagrams.len(),
        ..Default::default()
    };

    for (i, (name, diagram)) in diagrams.iter().enumerate() {
        let entry = validate_diagram(diagram, name, validator_path);

        if entry.rust_result.success {
            report.rust_rendered += 1;
        }
        if entry.ts_result.success {
            report.ts_rendered += 1;
        }
        if entry.rust_result.success && entry.ts_result.success {
            report.both_rendered += 1;
        }
        if let Some(ref comp) = entry.comparison {
            if comp.matches {
                report.matches += 1;
            }
        }

        report.entries.push(entry);

        if (i + 1) % 5 == 0 {
            eprint!("\rValidated {}/{} diagrams...", i + 1, report.total);
        }
    }
    eprintln!();

    report
}

fn print_report(report: &ValidationReport) {
    println!("\n{}", "=".repeat(60));
    println!("RENDER VALIDATION REPORT");
    println!("{}", "=".repeat(60));
    println!("Total diagrams:      {}", report.total);
    println!(
        "Rust rendered:       {} ({:.1}%)",
        report.rust_rendered,
        100.0 * report.rust_rendered as f64 / report.total as f64
    );
    println!(
        "TypeScript rendered: {} ({:.1}%)",
        report.ts_rendered,
        100.0 * report.ts_rendered as f64 / report.total as f64
    );
    println!("Both rendered:       {}", report.both_rendered);
    println!(
        "Structural matches:  {} ({:.1}% of both rendered)",
        report.matches,
        100.0 * report.matches as f64 / report.both_rendered.max(1) as f64
    );

    // Show differences
    let mismatches: Vec<_> = report
        .entries
        .iter()
        .filter(|e| e.comparison.as_ref().map(|c| !c.matches).unwrap_or(false))
        .collect();

    if !mismatches.is_empty() {
        println!("\nMismatches ({}):", mismatches.len());
        println!("{}", "-".repeat(60));
        for entry in mismatches.iter().take(10) {
            println!("\n[{}]", entry.name);
            if let Some(ref comp) = entry.comparison {
                println!("  Similarity: {:.1}%", comp.similarity * 100.0);
                for diff in &comp.critical_differences {
                    println!("  - {}", diff);
                }
            }
        }
    }

    // Show render failures
    let rust_failures: Vec<_> = report
        .entries
        .iter()
        .filter(|e| !e.rust_result.success && e.ts_result.success)
        .collect();

    if !rust_failures.is_empty() {
        println!("\nRust render failures ({}):", rust_failures.len());
        println!("{}", "-".repeat(60));
        for entry in rust_failures.iter().take(5) {
            println!("\n[{}]", entry.name);
            if let Some(ref err) = entry.rust_result.error {
                let err = if err.len() > 100 {
                    format!("{}...", &err[..100])
                } else {
                    err.clone()
                };
                println!("  Error: {}", err);
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: render-validate <diagram_file_or_dir> [--output results.json] [--validator-path path]");
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  render-validate test.mmd");
        eprintln!("  render-validate diagrams/");
        eprintln!("  render-validate diagrams.json --output results.json");
        std::process::exit(1);
    }

    let input_path = PathBuf::from(&args[1]);
    let mut output_path: Option<PathBuf> = None;
    let mut validator_path = std::env::current_dir()
        .unwrap_or_default()
        .join("tools/validation");

    // Parse arguments
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--output" | "-o" => {
                i += 1;
                if i < args.len() {
                    output_path = Some(PathBuf::from(&args[i]));
                }
            }
            "--validator-path" => {
                i += 1;
                if i < args.len() {
                    validator_path = PathBuf::from(&args[i]);
                }
            }
            _ => {}
        }
        i += 1;
    }

    // Load diagrams
    let diagrams = if input_path.is_dir() {
        // Load all .mmd files from directory
        let mut diagrams = Vec::new();
        for path in glob::glob(input_path.join("**/*.mmd").to_str().unwrap())
            .unwrap()
            .flatten()
        {
            if let Ok(content) = fs::read_to_string(&path) {
                let name = path.file_stem().unwrap().to_string_lossy().to_string();
                diagrams.push((name, content));
            }
        }
        diagrams
    } else if input_path.extension().map(|e| e == "json").unwrap_or(false) {
        // Load from JSON file (same format as cross-validate)
        #[derive(Deserialize)]
        struct DiagramEntry {
            diagram: String,
            #[serde(default)]
            test_name: String,
        }
        #[derive(Deserialize)]
        struct ExtractionResult {
            diagrams: Vec<DiagramEntry>,
        }

        let content = fs::read_to_string(&input_path).expect("Failed to read input file");
        let data: ExtractionResult = serde_json::from_str(&content).expect("Failed to parse JSON");
        data.diagrams
            .into_iter()
            .enumerate()
            .map(|(i, d)| {
                let name = if d.test_name.is_empty() {
                    format!("diagram_{}", i)
                } else {
                    d.test_name
                };
                (name, d.diagram)
            })
            .collect()
    } else {
        // Single file
        let content = fs::read_to_string(&input_path).expect("Failed to read input file");
        let name = input_path
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();
        vec![(name, content)]
    };

    eprintln!("Loaded {} diagrams", diagrams.len());

    // Filter to flowcharts only (what we can render)
    let flowcharts: Vec<_> = diagrams
        .into_iter()
        .filter(|(_, d)| {
            let lower = d.to_lowercase();
            lower.contains("flowchart") || lower.contains("graph ")
        })
        .collect();

    eprintln!("Found {} flowchart diagrams", flowcharts.len());

    if flowcharts.is_empty() {
        eprintln!("No flowchart diagrams found to validate");
        std::process::exit(0);
    }

    // Run validation
    let report = run_validation(&flowcharts, &validator_path);

    // Print report
    print_report(&report);

    // Save detailed results
    if let Some(output) = output_path {
        let json = serde_json::to_string_pretty(&report).unwrap();
        fs::write(&output, &json).expect("Failed to write output");
        eprintln!("\nDetailed results saved to {}", output.display());
    }
}
