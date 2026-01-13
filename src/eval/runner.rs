//! Evaluation pipeline runner.
//!
//! This module orchestrates the evaluation process:
//! 1. Parse diagram with selkie
//! 2. Render diagram with selkie
//! 3. Get reference SVG (from cache or render with mermaid.js)
//! 4. Run structural checks
//! 5. Calculate visual similarity (SSIM)
//! 6. Compile results

use super::cache::ReferenceCache;
use super::checks::{check_structure, CheckConfig};
use super::samples::Sample;
use super::{
    DiagramResult, Dimensions, EvalResult, Issue, Level, ParseResult, RenderResult, Status,
};
use crate::render::svg::SvgStructure;

/// Configuration for the evaluation runner
#[derive(Debug, Clone, Default)]
pub struct EvalConfig {
    /// Filter by diagram type (None = all types)
    pub diagram_type_filter: Option<String>,
    /// Whether to skip visual comparison (SSIM)
    pub skip_visual: bool,
    /// Whether to force refresh cached references
    pub force_refresh: bool,
    /// Structural check configuration
    pub check_config: CheckConfig,
}

/// Input diagram for evaluation
#[derive(Debug, Clone)]
pub struct DiagramInput {
    /// Name/identifier
    pub name: String,
    /// Source file (optional)
    pub source: Option<String>,
    /// Diagram type (auto-detected if not provided)
    pub diagram_type: Option<String>,
    /// The mermaid diagram source text
    pub text: String,
}

impl From<Sample> for DiagramInput {
    fn from(sample: Sample) -> Self {
        Self {
            name: sample.name.to_string(),
            source: None,
            diagram_type: Some(sample.diagram_type.to_string()),
            text: sample.source.to_string(),
        }
    }
}

/// Evaluation runner
pub struct EvalRunner {
    config: EvalConfig,
    cache: ReferenceCache,
    /// SVG pairs collected during evaluation (for PNG generation)
    svg_pairs: std::cell::RefCell<Vec<(String, String, String)>>, // (name, selkie_svg, reference_svg)
}

impl EvalRunner {
    /// Create a new runner with configuration
    pub fn new(config: EvalConfig, cache: ReferenceCache) -> Self {
        Self {
            config,
            cache,
            svg_pairs: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// Create a runner with default configuration
    pub fn with_defaults() -> Self {
        Self::new(EvalConfig::default(), ReferenceCache::with_defaults())
    }

    /// Get collected SVG pairs for PNG generation
    pub fn take_svg_pairs(&self) -> Vec<(String, String, String)> {
        self.svg_pairs.borrow_mut().drain(..).collect()
    }

    /// Evaluate a list of diagrams
    pub fn evaluate(&self, inputs: &[DiagramInput]) -> EvalResult {
        let mut result = EvalResult::new();

        // Apply type filter
        let filtered: Vec<_> = inputs
            .iter()
            .filter(|input| {
                if let Some(ref filter) = self.config.diagram_type_filter {
                    input.diagram_type.as_deref() == Some(filter.as_str())
                } else {
                    true
                }
            })
            .collect();

        // Evaluate each diagram
        for (i, input) in filtered.iter().enumerate() {
            eprint!(
                "\rEvaluating {}/{}: {}...",
                i + 1,
                filtered.len(),
                input.name
            );
            let diagram_result = self.evaluate_single(input);
            result.diagrams.push(diagram_result);
        }
        eprintln!();

        // Compute summary statistics
        result.compute_stats();

        result
    }

    /// Evaluate a single diagram
    pub fn evaluate_single(&self, input: &DiagramInput) -> DiagramResult {
        let mut result = DiagramResult {
            name: input.name.clone(),
            source: input.source.clone(),
            diagram_type: input
                .diagram_type
                .clone()
                .unwrap_or_else(|| detect_diagram_type(&input.text)),
            diagram_text: Some(input.text.clone()),
            status: Status::Match,
            visual_similarity: None,
            structural_match: true,
            issues: Vec::new(),
            parse_result: ParseResult {
                selkie_success: false,
                selkie_error: None,
                reference_success: false,
                reference_error: None,
            },
            render_result: None,
        };

        // Step 1: Parse with selkie
        let selkie_parsed = crate::parse(&input.text);
        match &selkie_parsed {
            Ok(_) => result.parse_result.selkie_success = true,
            Err(e) => {
                result.parse_result.selkie_error = Some(e.to_string());
                result
                    .issues
                    .push(Issue::error("parse", format!("Selkie parse failed: {}", e)));
            }
        }

        // Step 2: Get reference SVG
        let reference_svg = if self.config.force_refresh {
            self.cache.render_with_mermaid(&input.text)
        } else {
            self.cache.get_or_render(&input.text)
        };

        match &reference_svg {
            Ok(_) => result.parse_result.reference_success = true,
            Err(e) => {
                result.parse_result.reference_error = Some(e.clone());
                // Not necessarily an error - mermaid.js might legitimately reject it
            }
        }

        // If neither parser succeeded, we're done
        if !result.parse_result.selkie_success && !result.parse_result.reference_success {
            result.status = result.determine_status();
            return result;
        }

        // Step 3: Render with selkie (if parsed)
        let selkie_svg = if let Ok(parsed) = &selkie_parsed {
            match crate::render(parsed) {
                Ok(svg) => Some(svg),
                Err(e) => {
                    result.issues.push(Issue::error(
                        "render",
                        format!("Selkie render failed: {}", e),
                    ));
                    None
                }
            }
        } else {
            None
        };

        // Step 4: Extract structures and compare
        let selkie_structure = selkie_svg
            .as_ref()
            .and_then(|svg| SvgStructure::from_svg(svg).ok());
        let reference_structure = reference_svg
            .as_ref()
            .ok()
            .and_then(|svg| SvgStructure::from_svg(svg).ok());

        // Update render result
        result.render_result = Some(RenderResult {
            selkie_success: selkie_svg.is_some(),
            selkie_error: if selkie_svg.is_none() && result.parse_result.selkie_success {
                Some("Render failed".to_string())
            } else {
                None
            },
            reference_success: reference_structure.is_some(),
            reference_error: if reference_structure.is_none()
                && result.parse_result.reference_success
            {
                Some("Structure extraction failed".to_string())
            } else {
                None
            },
            selkie_dimensions: selkie_structure.as_ref().map(|s| Dimensions {
                width: s.width,
                height: s.height,
            }),
            reference_dimensions: reference_structure.as_ref().map(|s| Dimensions {
                width: s.width,
                height: s.height,
            }),
        });

        // Step 5: Structural comparison
        if let (Some(selkie_struct), Some(ref_struct)) = (&selkie_structure, &reference_structure) {
            let check_issues =
                check_structure(selkie_struct, ref_struct, &self.config.check_config);
            result.structural_match = !check_issues.iter().any(|i| i.level == Level::Error);
            result.issues.extend(check_issues);
        }

        // Step 6: Visual similarity (SSIM) and collect SVG pairs
        if let (Some(selkie), Ok(reference)) = (&selkie_svg, &reference_svg) {
            // Store SVG pair for PNG generation
            self.svg_pairs.borrow_mut().push((
                input.name.clone(),
                selkie.clone(),
                reference.clone(),
            ));

            // Calculate visual similarity if not skipped
            if !self.config.skip_visual {
                match super::png::compare_svgs(selkie, reference) {
                    Ok(comparison) => {
                        result.visual_similarity = Some(comparison.ssim);
                    }
                    Err(_e) => {
                        // Visual comparison failed (e.g., png feature not enabled)
                        // This is not a structural error, so we don't add an issue
                    }
                }
            }
        }

        // Determine final status
        result.status = result.determine_status();

        result
    }
}

/// Detect diagram type from text
fn detect_diagram_type(text: &str) -> String {
    let text_lower = text.to_lowercase();

    // Simple substring matching for diagram type detection
    if text_lower.contains("sequencediagram") {
        return "sequence".to_string();
    }
    if text_lower.contains("classdiagram") {
        return "class".to_string();
    }
    if text_lower.contains("statediagram") {
        return "state".to_string();
    }
    if text_lower.contains("erdiagram") {
        return "er".to_string();
    }
    if text_lower.contains("gitgraph") {
        return "git".to_string();
    }
    if text_lower.contains("gantt") {
        return "gantt".to_string();
    }
    if text_lower.contains("pie") {
        return "pie".to_string();
    }
    if text_lower.contains("mindmap") {
        return "mindmap".to_string();
    }
    if text_lower.contains("timeline") {
        return "timeline".to_string();
    }
    if text_lower.contains("journey") {
        return "journey".to_string();
    }
    if text_lower.contains("quadrantchart") {
        return "quadrant".to_string();
    }
    if text_lower.contains("xychart") {
        return "xychart".to_string();
    }
    if text_lower.contains("sankey") {
        return "sankey".to_string();
    }
    if text_lower.contains("packet") {
        return "packet".to_string();
    }
    if text_lower.contains("block") {
        return "block".to_string();
    }
    if text_lower.contains("flowchart") {
        return "flowchart".to_string();
    }
    // "graph" followed by whitespace indicates flowchart
    if text_lower.starts_with("graph ")
        || text_lower.starts_with("graph\t")
        || text_lower.starts_with("graph\n")
    {
        return "flowchart".to_string();
    }

    "unknown".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_diagram_type() {
        assert_eq!(detect_diagram_type("flowchart LR\nA-->B"), "flowchart");
        assert_eq!(detect_diagram_type("graph TD\nA-->B"), "flowchart");
        assert_eq!(
            detect_diagram_type("sequenceDiagram\nA->>B: Hi"),
            "sequence"
        );
        assert_eq!(detect_diagram_type("pie\n\"A\": 50"), "pie");
    }

    #[test]
    fn test_diagram_input_from_sample() {
        let sample = Sample {
            name: "test",
            diagram_type: "flowchart",
            source: "flowchart LR\n    A --> B",
        };
        let input: DiagramInput = sample.into();
        assert_eq!(input.name, "test");
        assert_eq!(input.diagram_type, Some("flowchart".to_string()));
    }
}
