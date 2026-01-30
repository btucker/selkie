//! TUI renderer for Sankey flow diagrams.
//!
//! Since flow-width proportional rendering is complex in character art,
//! Sankey diagrams are displayed as a flow table showing source → target
//! relationships with proportional bar widths.

use crate::diagrams::sankey::SankeyDb;
use crate::error::Result;

const BAR_WIDTH: usize = 30;
const FULL_BLOCK: char = '█';
const HALF_BLOCK: char = '▌';

/// Render a Sankey diagram as character art.
pub fn render_sankey_tui(db: &SankeyDb) -> Result<String> {
    let links = db.get_links();
    if links.is_empty() {
        return Ok("(empty sankey diagram)\n".to_string());
    }

    let mut lines: Vec<String> = Vec::new();
    lines.push("Flow Diagram".to_string());
    lines.push("─".repeat(40));

    // Find max flow value for scaling
    let max_value = links.iter().map(|l| l.value).fold(0.0f64, f64::max);

    if max_value <= 0.0 {
        lines.push("(no data)".to_string());
        lines.push(String::new());
        return Ok(lines.join("\n"));
    }

    // Find max label widths
    let max_source_len = links
        .iter()
        .map(|l| l.source.chars().count())
        .max()
        .unwrap_or(5);
    let max_target_len = links
        .iter()
        .map(|l| l.target.chars().count())
        .max()
        .unwrap_or(5);

    // Render each flow
    for link in links {
        let pct = link.value / max_value;
        let bar_cells = pct * BAR_WIDTH as f64;
        let full_cells = bar_cells.floor() as usize;
        let has_half = (bar_cells - full_cells as f64) >= 0.5;

        let mut bar = String::new();
        for _ in 0..full_cells {
            bar.push(FULL_BLOCK);
        }
        if has_half {
            bar.push(HALF_BLOCK);
        }
        if bar.is_empty() && link.value > 0.0 {
            bar.push(HALF_BLOCK);
        }

        let value_str = if link.value.fract() == 0.0 {
            format!("{}", link.value as i64)
        } else {
            format!("{:.1}", link.value)
        };

        lines.push(format!(
            "  {:sw$} → {:tw$} │{} {}",
            link.source,
            link.target,
            bar,
            value_str,
            sw = max_source_len,
            tw = max_target_len,
        ));
    }

    // Summary: total flow per source node
    let nodes = db.get_nodes();
    let mut source_totals: Vec<(&str, f64)> = Vec::new();
    for node in nodes {
        let total: f64 = links
            .iter()
            .filter(|l| l.source == node.id)
            .map(|l| l.value)
            .sum();
        if total > 0.0 {
            source_totals.push((&node.id, total));
        }
    }

    if !source_totals.is_empty() {
        lines.push(String::new());
        lines.push("  Totals:".to_string());
        for (name, total) in &source_totals {
            let total_str = if total.fract() == 0.0 {
                format!("{}", *total as i64)
            } else {
                format!("{:.1}", total)
            };
            lines.push(format!("    {} = {}", name, total_str));
        }
    }

    lines.push(String::new());
    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_sankey() {
        let db = SankeyDb::new();
        let output = render_sankey_tui(&db).unwrap();
        assert!(output.contains("empty sankey"));
    }

    #[test]
    fn gallery_sankey_renders() {
        let input = std::fs::read_to_string("docs/sources/sankey.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Sankey(db) => db,
            _ => panic!("Expected sankey"),
        };
        let output = render_sankey_tui(&db).unwrap();
        assert!(output.contains("Revenue"), "Output:\n{}", output);
        assert!(output.contains("Salaries"), "Output:\n{}", output);
        assert!(output.contains("Operations"), "Output:\n{}", output);
    }

    #[test]
    fn flow_arrows_present() {
        let input = std::fs::read_to_string("docs/sources/sankey.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Sankey(db) => db,
            _ => panic!("Expected sankey"),
        };
        let output = render_sankey_tui(&db).unwrap();
        assert!(
            output.contains('→'),
            "Should have flow arrows\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn bars_proportional() {
        let input = std::fs::read_to_string("docs/sources/sankey.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Sankey(db) => db,
            _ => panic!("Expected sankey"),
        };
        let output = render_sankey_tui(&db).unwrap();
        // Salaries (40) should have more blocks than Profit (8)
        let salary_line = output.lines().find(|l| l.contains("Salaries")).unwrap();
        let profit_line = output.lines().find(|l| l.contains("Profit")).unwrap();
        let count_blocks = |line: &str| -> usize {
            line.chars()
                .filter(|&c| c == FULL_BLOCK || c == HALF_BLOCK)
                .count()
        };
        assert!(
            count_blocks(salary_line) > count_blocks(profit_line),
            "Salaries should have more blocks than Profit"
        );
    }
}
