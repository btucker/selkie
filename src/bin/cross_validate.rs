//! Cross-validate Rust mermaid parsers against the reference TypeScript implementation.
//!
//! This tool:
//! 1. Loads diagram examples from a JSON file or extracts them from test files
//! 2. Parses each diagram with both Rust and TypeScript implementations
//! 3. Reports discrepancies and generates a compatibility report

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Serialize, Deserialize)]
struct DiagramEntry {
    diagram: String,
    source_file: String,
    test_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExtractionResult {
    count: usize,
    diagrams: Vec<DiagramEntry>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct ValidationResult {
    diagram: String,
    source_file: String,
    test_name: String,
    rust_valid: bool,
    rust_error: String,
    rust_type: String,
    ts_valid: bool,
    ts_error: String,
    ts_type: String,
    #[serde(rename = "match")]
    matches: bool,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct ValidationReport {
    total: usize,
    rust_passed: usize,
    ts_passed: usize,
    both_passed: usize,
    both_failed: usize,
    rust_only: usize,
    ts_only: usize,
    results: Vec<ValidationResult>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TsValidationResult {
    valid: bool,
    #[serde(rename = "diagramType")]
    diagram_type: Option<String>,
    error: Option<String>,
}

/// Map diagram text patterns to parser type names
fn detect_diagram_type(text: &str) -> Option<&'static str> {
    let patterns: &[(&str, &str)] = &[
        // Specific diagram types first
        (r"sequenceDiagram", "sequence"),
        (r"classDiagram", "class"),
        (r"stateDiagram", "state"),
        (r"erDiagram", "er"),
        (r"gitGraph", "git"),
        (r"requirementDiagram", "requirement"),
        (r"quadrantChart", "quadrant"),
        // Beta diagram types
        (r"xychart-beta", "xychart"),
        (r"xychart", "xychart"),
        (r"sankey-beta", "sankey"),
        (r"sankey", "sankey"),
        (r"packet-beta", "packet"),
        (r"packet", "packet"),
        (r"block-beta", "block"),
        (r"architecture-beta", "architecture"),
        (r"radar-beta", "radar"),
        (r"treemap-beta", "treemap"),
        (r"treemap", "treemap"),
        // C4 variants
        (r"C4Context", "c4"),
        (r"C4Container", "c4"),
        (r"C4Component", "c4"),
        (r"C4Dynamic", "c4"),
        (r"C4Deployment", "c4"),
        // Single keyword types
        (r"gantt", "gantt"),
        (r"pie", "pie"),
        (r"mindmap", "mindmap"),
        (r"timeline", "timeline"),
        (r"journey", "journey"),
        (r"kanban", "kanban"),
        (r"info", "info"),
        // Flowchart variants (last, as "graph" is generic)
        (r"flowchart", "flowchart"),
        (r"graph[\s\n]", "flowchart"),
    ];

    // Strip frontmatter and directives
    let clean = strip_frontmatter_and_directives(text);

    for (pattern, dtype) in patterns {
        if let Ok(regex) = Regex::new(&format!("(?i){}", pattern)) {
            if regex.is_match(&clean) {
                return Some(dtype);
            }
        }
    }
    None
}

/// Strip YAML frontmatter and mermaid directives
fn strip_frontmatter_and_directives(text: &str) -> String {
    let mut result = text.to_string();

    // Strip YAML frontmatter
    let frontmatter_re = Regex::new(r"(?s)^\s*---\n.*?\n\s*---\n?").unwrap();
    result = frontmatter_re.replace(&result, "").to_string();

    // Strip mermaid directives
    let directive_re = Regex::new(r"(?s)%%\{.*?\}%%\n?").unwrap();
    result = directive_re.replace_all(&result, "").to_string();

    result.trim().to_string()
}

/// Validate diagram with Rust parser
fn validate_with_rust(text: &str) -> (bool, String, String) {
    let clean_text = strip_frontmatter_and_directives(text);

    let dtype = match detect_diagram_type(&clean_text) {
        Some(t) => t,
        None => return (false, "Unknown diagram type".to_string(), String::new()),
    };

    let result: Result<(), String> = match dtype {
        "sequence" => mermaid::diagrams::sequence::parse(&clean_text)
            .map(|_| ())
            .map_err(|e| e.to_string()),
        "flowchart" => mermaid::diagrams::flowchart::parse(&clean_text)
            .map(|_| ())
            .map_err(|e| e.to_string()),
        "class" => mermaid::diagrams::class::parse(&clean_text)
            .map(|_| ())
            .map_err(|e| e.to_string()),
        "state" => mermaid::diagrams::state::parse(&clean_text)
            .map(|_| ())
            .map_err(|e| e.to_string()),
        "er" => mermaid::diagrams::er::parse(&clean_text)
            .map(|_| ())
            .map_err(|e| e.to_string()),
        "gantt" => mermaid::diagrams::gantt::parse(&clean_text)
            .map(|_| ())
            .map_err(|e| e.to_string()),
        "pie" => mermaid::diagrams::pie::parse(&clean_text)
            .map(|_| ())
            .map_err(|e| e.to_string()),
        "mindmap" => mermaid::diagrams::mindmap::parse(&clean_text)
            .map(|_| ())
            .map_err(|e| e.to_string()),
        "timeline" => mermaid::diagrams::timeline::parse(&clean_text)
            .map(|_| ())
            .map_err(|e| e.to_string()),
        "journey" => mermaid::diagrams::journey::parse(&clean_text)
            .map(|_| ())
            .map_err(|e| e.to_string()),
        "quadrant" => mermaid::diagrams::quadrant::parse(&clean_text)
            .map(|_| ())
            .map_err(|e| e.to_string()),
        "xychart" => mermaid::diagrams::xychart::parse(&clean_text)
            .map(|_| ())
            .map_err(|e| e.to_string()),
        "sankey" => mermaid::diagrams::sankey::parse(&clean_text)
            .map(|_| ())
            .map_err(|e| e.to_string()),
        "packet" => mermaid::diagrams::packet::parse(&clean_text)
            .map(|_| ())
            .map_err(|e| e.to_string()),
        "block" => mermaid::diagrams::block::parse(&clean_text)
            .map(|_| ())
            .map_err(|e| e.to_string()),
        "architecture" => mermaid::diagrams::architecture::parse(&clean_text)
            .map(|_| ())
            .map_err(|e| e.to_string()),
        "c4" => mermaid::diagrams::c4::parse(&clean_text)
            .map(|_| ())
            .map_err(|e| e.to_string()),
        "git" => mermaid::diagrams::git::parse(&clean_text)
            .map(|_| ())
            .map_err(|e| e.to_string()),
        "requirement" => mermaid::diagrams::requirement::parse(&clean_text)
            .map(|_| ())
            .map_err(|e| e.to_string()),
        "kanban" => mermaid::diagrams::kanban::parse(&clean_text)
            .map(|_| ())
            .map_err(|e| e.to_string()),
        "info" => mermaid::diagrams::info::parse(&clean_text)
            .map(|_| ())
            .map_err(|e| e.to_string()),
        "radar" => mermaid::diagrams::radar::parse(&clean_text)
            .map(|_| ())
            .map_err(|e| e.to_string()),
        "treemap" => mermaid::diagrams::treemap::parse(&clean_text)
            .map(|_| ())
            .map_err(|e| e.to_string()),
        _ => {
            return (
                false,
                format!("No Rust parser for type: {}", dtype),
                dtype.to_string(),
            )
        }
    };

    match result {
        Ok(_) => (true, String::new(), dtype.to_string()),
        Err(e) => (false, e.to_string(), dtype.to_string()),
    }
}

/// Validate diagram with TypeScript mermaid library
fn validate_with_typescript(text: &str, validator_path: &Path) -> (bool, String, String) {
    let validator_script = validator_path.join("validate_mermaid.mjs");

    let mut child = match Command::new("node")
        .arg(&validator_script)
        .arg("-")
        .current_dir(validator_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return (false, format!("Failed to spawn node: {}", e), String::new()),
    };

    // Write diagram to stdin
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(text.as_bytes());
    }

    let output = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => return (false, format!("Failed to get output: {}", e), String::new()),
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return (false, stderr.to_string(), String::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    match serde_json::from_str::<TsValidationResult>(&stdout) {
        Ok(result) => {
            if result.valid {
                (true, String::new(), result.diagram_type.unwrap_or_default())
            } else {
                (false, result.error.unwrap_or_default(), String::new())
            }
        }
        Err(e) => (
            false,
            format!("Invalid JSON: {} - {}", e, stdout),
            String::new(),
        ),
    }
}

fn validate_diagram(
    diagram: &DiagramEntry,
    validator_path: &Path,
    skip_ts: bool,
) -> ValidationResult {
    let (rust_valid, rust_error, rust_type) = validate_with_rust(&diagram.diagram);

    let (ts_valid, ts_error, ts_type) = if skip_ts {
        (false, "Skipped".to_string(), String::new())
    } else {
        validate_with_typescript(&diagram.diagram, validator_path)
    };

    let matches = rust_valid == ts_valid;

    ValidationResult {
        diagram: if diagram.diagram.len() > 200 {
            format!("{}...", &diagram.diagram[..200])
        } else {
            diagram.diagram.clone()
        },
        source_file: diagram.source_file.clone(),
        test_name: diagram.test_name.clone(),
        rust_valid,
        rust_error,
        rust_type,
        ts_valid,
        ts_error,
        ts_type,
        matches,
    }
}

fn run_validation(
    diagrams: &[DiagramEntry],
    validator_path: &Path,
    skip_ts: bool,
) -> ValidationReport {
    let mut report = ValidationReport {
        total: diagrams.len(),
        ..Default::default()
    };

    for (i, diagram) in diagrams.iter().enumerate() {
        let result = validate_diagram(diagram, validator_path, skip_ts);

        if result.rust_valid {
            report.rust_passed += 1;
        }
        if result.ts_valid {
            report.ts_passed += 1;
        }

        if result.rust_valid && result.ts_valid {
            report.both_passed += 1;
        } else if !result.rust_valid && !result.ts_valid {
            report.both_failed += 1;
        } else if result.rust_valid && !result.ts_valid {
            report.rust_only += 1;
        } else {
            report.ts_only += 1;
        }

        report.results.push(result);

        // Progress indicator
        if (i + 1) % 10 == 0 {
            eprint!("\rValidated {}/{} diagrams...", i + 1, report.total);
        }
    }
    eprintln!();

    report
}

fn print_report(report: &ValidationReport, verbose: bool) {
    println!("\n{}", "=".repeat(60));
    println!("CROSS-VALIDATION REPORT");
    println!("{}", "=".repeat(60));
    println!("Total diagrams:     {}", report.total);
    println!(
        "Rust passed:        {} ({:.1}%)",
        report.rust_passed,
        100.0 * report.rust_passed as f64 / report.total as f64
    );
    println!(
        "TypeScript passed:  {} ({:.1}%)",
        report.ts_passed,
        100.0 * report.ts_passed as f64 / report.total as f64
    );
    println!("Both passed:        {}", report.both_passed);
    println!("Both failed:        {}", report.both_failed);
    println!("Rust only:          {}", report.rust_only);
    println!("TypeScript only:    {}", report.ts_only);

    // Show discrepancies
    let discrepancies: Vec<&ValidationResult> =
        report.results.iter().filter(|r| !r.matches).collect();
    if !discrepancies.is_empty() {
        println!("\nDiscrepancies ({}):", discrepancies.len());
        println!("{}", "-".repeat(60));
        for r in discrepancies.iter().take(20) {
            let status = if r.rust_valid {
                "RS✓ TS✗"
            } else {
                "RS✗ TS✓"
            };
            println!("\n[{}] {}", status, r.test_name);
            println!("  Source: {}", r.source_file);
            if !r.rust_error.is_empty() {
                let err = if r.rust_error.len() > 100 {
                    format!("{}...", &r.rust_error[..100])
                } else {
                    r.rust_error.clone()
                };
                println!("  Rust error: {}", err);
            }
            if !r.ts_error.is_empty() {
                let err = if r.ts_error.len() > 100 {
                    format!("{}...", &r.ts_error[..100])
                } else {
                    r.ts_error.clone()
                };
                println!("  TS error: {}", err);
            }
            if verbose {
                let diag = if r.diagram.len() > 100 {
                    format!("{}...", &r.diagram[..100])
                } else {
                    r.diagram.clone()
                };
                println!("  Diagram: {}", diag);
            }
        }
    }

    // Show breakdown by diagram type
    println!("\n\nBreakdown by diagram type:");
    println!("{}", "-".repeat(60));
    let mut by_type: HashMap<String, (usize, usize, usize)> = HashMap::new();
    for r in &report.results {
        let dtype = if !r.rust_type.is_empty() {
            r.rust_type.clone()
        } else if !r.ts_type.is_empty() {
            r.ts_type.clone()
        } else {
            "unknown".to_string()
        };
        let entry = by_type.entry(dtype).or_insert((0, 0, 0));
        entry.0 += 1; // total
        if r.rust_valid {
            entry.1 += 1; // rust passed
        }
        if r.ts_valid {
            entry.2 += 1; // ts passed
        }
    }
    let mut types: Vec<_> = by_type.iter().collect();
    types.sort_by_key(|(k, _)| k.as_str());
    for (dtype, (total, rust, ts)) in types {
        println!(
            "  {:<15} total: {:>3}, rust: {:>3} ({:>5.1}%), ts: {:>3} ({:>5.1}%)",
            dtype,
            total,
            rust,
            100.0 * *rust as f64 / *total as f64,
            ts,
            100.0 * *ts as f64 / *total as f64
        );
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: cross-validate <diagrams.json> [--skip-ts] [--output results.json] [--verbose] [--validator-path path]");
        std::process::exit(1);
    }

    let input_path = PathBuf::from(&args[1]);
    let mut output_path: Option<PathBuf> = None;
    let mut skip_ts = false;
    let mut verbose = false;
    // Default to tools/validation relative to current working directory
    let mut validator_path = std::env::current_dir()
        .unwrap_or_default()
        .join("tools/validation");

    // Parse arguments
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--skip-ts" => skip_ts = true,
            "--verbose" | "-v" => verbose = true,
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
    let content = fs::read_to_string(&input_path).expect("Failed to read input file");
    let data: ExtractionResult = serde_json::from_str(&content).expect("Failed to parse JSON");

    eprintln!("Loaded {} diagrams", data.diagrams.len());

    // Run validation
    let report = run_validation(&data.diagrams, &validator_path, skip_ts);

    // Print report
    print_report(&report, verbose);

    // Save detailed results
    if let Some(output) = output_path {
        let json = serde_json::to_string_pretty(&report).unwrap();
        fs::write(&output, &json).expect("Failed to write output");
        eprintln!("\nDetailed results saved to {}", output.display());
    }
}
