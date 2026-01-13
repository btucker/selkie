//! Pie chart parser

use regex::Regex;
use std::sync::LazyLock;

use super::types::PieDb;
use crate::error::{MermaidError, Result};

// Regex patterns for pie chart parsing
static PIE_HEADER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*pie\s*(showData)?(?:\s+title\s+(.+))?$").unwrap());

static SECTION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^\s*"([^"]+)"\s*:\s*(-?\d+(?:\.\d+)?)\s*$"#).unwrap());

static ACC_TITLE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*accTitle\s*:\s*(.+)$").unwrap());

static ACC_DESCR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*accDescr\s*:\s*(.+)$").unwrap());

static ACC_DESCR_MULTILINE_START_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*accDescr\s*\{").unwrap());

static COMMENT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*%%").unwrap());

/// Parse a pie chart diagram
pub fn parse(input: &str) -> Result<PieDb> {
    let mut db = PieDb::new();
    parse_into(input, &mut db)?;
    Ok(db)
}

/// Parse into an existing database (for testing)
pub fn parse_into(input: &str, db: &mut PieDb) -> Result<()> {
    db.clear();

    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;

    // Parse header
    while i < lines.len() {
        let line = lines[i];

        // Skip empty lines and comments
        if line.trim().is_empty() || COMMENT_RE.is_match(line) {
            i += 1;
            continue;
        }

        // Look for pie header
        if let Some(caps) = PIE_HEADER_RE.captures(line) {
            if caps.get(1).is_some() {
                db.set_show_data(true);
            }
            if let Some(title) = caps.get(2) {
                db.set_diagram_title(title.as_str().trim());
            }
            i += 1;
            break;
        }

        i += 1;
    }

    // Parse body
    let mut in_multiline_descr = false;
    let mut multiline_descr = Vec::new();

    while i < lines.len() {
        let line = lines[i];

        // Handle multiline accDescr
        if in_multiline_descr {
            if line.trim() == "}" {
                in_multiline_descr = false;
                db.set_acc_description(multiline_descr.join("\n"));
                multiline_descr.clear();
            } else {
                multiline_descr.push(line.trim());
            }
            i += 1;
            continue;
        }

        // Skip empty lines and comments
        if line.trim().is_empty() || COMMENT_RE.is_match(line) {
            i += 1;
            continue;
        }

        // Check for accTitle
        if let Some(caps) = ACC_TITLE_RE.captures(line) {
            db.set_acc_title(caps.get(1).unwrap().as_str().trim());
            i += 1;
            continue;
        }

        // Check for single-line accDescr
        if let Some(caps) = ACC_DESCR_RE.captures(line) {
            db.set_acc_description(caps.get(1).unwrap().as_str().trim());
            i += 1;
            continue;
        }

        // Check for multiline accDescr start
        if ACC_DESCR_MULTILINE_START_RE.is_match(line) {
            in_multiline_descr = true;
            i += 1;
            continue;
        }

        // Check for section
        if let Some(caps) = SECTION_RE.captures(line) {
            let label = caps.get(1).unwrap().as_str();
            let value: f64 = caps.get(2).unwrap().as_str().parse().map_err(|_| {
                MermaidError::ParseError(format!("Invalid number in pie chart: {}", line))
            })?;

            db.add_section(label, value)?;
        }

        i += 1;
    }

    Ok(())
}
