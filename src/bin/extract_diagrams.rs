//! Extract mermaid diagram examples from TypeScript test files.
//!
//! This tool parses TypeScript/JavaScript test files and extracts
//! mermaid diagram definitions from template literals.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

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

/// Diagram type keywords to look for in template literals
const DIAGRAM_TYPES: &[&str] = &[
    "sequenceDiagram",
    "flowchart",
    "graph",
    "classDiagram",
    "stateDiagram",
    "erDiagram",
    "gantt",
    "pie",
    "mindmap",
    "timeline",
    "journey",
    "quadrantChart",
    "xychart-beta",
    "sankey-beta",
    "packet-beta",
    "block-beta",
    "architecture-beta",
    "C4Context",
    "C4Container",
    "C4Component",
    "gitGraph",
    "requirementDiagram",
    "kanban",
    "info",
    "radar-beta",
    "treemap-beta",
];

fn extract_diagrams_from_file(filepath: &Path) -> Vec<DiagramEntry> {
    let content = match fs::read_to_string(filepath) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut diagrams = Vec::new();

    // Build pattern to match any diagram type
    let type_pattern = DIAGRAM_TYPES
        .iter()
        .map(|t| regex::escape(t))
        .collect::<Vec<_>>()
        .join("|");
    let type_regex = Regex::new(&format!("(?i){}", type_pattern)).unwrap();

    // Pattern to find template literals (backtick strings)
    let template_regex = Regex::new(r"`([^`]+)`").unwrap();

    // Pattern to find test names
    let test_regex = Regex::new(r#"(?i)(?:it|test)\s*\(\s*['"]([^'"]+)['"]"#).unwrap();

    // Track current test name by position
    let test_positions: Vec<(usize, String)> = test_regex
        .captures_iter(&content)
        .map(|cap| (cap.get(0).unwrap().start(), cap[1].to_string()))
        .collect();

    for cap in template_regex.captures_iter(&content) {
        let template_content = &cap[1];

        // Check if this template contains a diagram type
        if !type_regex.is_match(template_content) {
            continue;
        }

        let diagram_text = template_content.trim().to_string();

        // Skip if it's too short (probably a partial reference)
        if diagram_text.len() < 20 {
            continue;
        }

        // Find the test name for this diagram
        let pos = cap.get(0).unwrap().start();
        let test_name = test_positions
            .iter()
            .filter(|(p, _)| *p < pos)
            .last()
            .map(|(_, name)| name.clone())
            .unwrap_or_else(|| "unknown".to_string());

        diagrams.push(DiagramEntry {
            diagram: diagram_text,
            source_file: filepath.to_string_lossy().to_string(),
            test_name,
        });
    }

    diagrams
}

fn extract_from_directory(directory: &Path) -> Vec<DiagramEntry> {
    let mut all_diagrams = Vec::new();

    let patterns = [
        "**/*.spec.ts",
        "**/*.spec.js",
        "**/*.test.ts",
        "**/*.test.js",
    ];

    for pattern in patterns {
        let full_pattern = directory.join(pattern);
        if let Ok(paths) = glob::glob(full_pattern.to_str().unwrap_or("")) {
            for entry in paths.flatten() {
                let diagrams = extract_diagrams_from_file(&entry);
                all_diagrams.extend(diagrams);
            }
        }
    }

    all_diagrams
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: extract-diagrams <directory_or_file> [output.json]");
        std::process::exit(1);
    }

    let input_path = PathBuf::from(&args[1]);
    let output_path = args.get(2).map(PathBuf::from);

    let diagrams = if input_path.is_file() {
        extract_diagrams_from_file(&input_path)
    } else if input_path.is_dir() {
        extract_from_directory(&input_path)
    } else {
        eprintln!("Error: {} does not exist", input_path.display());
        std::process::exit(1);
    };

    // Remove duplicates based on diagram content
    let mut seen = HashSet::new();
    let unique_diagrams: Vec<DiagramEntry> = diagrams
        .into_iter()
        .filter(|d| {
            let key = d.diagram.trim().to_string();
            if seen.contains(&key) {
                false
            } else {
                seen.insert(key);
                true
            }
        })
        .collect();

    let result = ExtractionResult {
        count: unique_diagrams.len(),
        diagrams: unique_diagrams,
    };

    let json = serde_json::to_string_pretty(&result).unwrap();

    if let Some(output) = output_path {
        fs::write(&output, &json).expect("Failed to write output file");
        eprintln!(
            "Extracted {} diagrams to {}",
            result.count,
            output.display()
        );
    } else {
        println!("{}", json);
    }
}
