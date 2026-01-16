//! Structural checks for comparing SVG outputs.
//!
//! This module defines the 3-level check system:
//! - Error: Structural breaks (node/edge count mismatch, missing labels)
//! - Warning: Significant differences (dimensions >20% off, shape counts differ)
//! - Info: Acceptable variations (styling, minor dimension differences)

use super::Issue;
use crate::render::svg::SvgStructure;
use std::collections::HashSet;

/// Configuration for structural checks
#[derive(Debug, Clone)]
pub struct CheckConfig {
    /// Dimension difference threshold for warnings (percentage, e.g., 0.2 = 20%)
    pub dimension_warning_threshold: f64,
    /// Dimension difference threshold for info (percentage, e.g., 0.05 = 5%)
    pub dimension_info_threshold: f64,
}

impl Default for CheckConfig {
    fn default() -> Self {
        Self {
            dimension_warning_threshold: 0.20, // 20%
            dimension_info_threshold: 0.05,    // 5%
        }
    }
}

/// Run all structural checks between selkie and reference SVGs
pub fn check_structure(
    selkie: &SvgStructure,
    reference: &SvgStructure,
    config: &CheckConfig,
) -> Vec<Issue> {
    let mut issues = Vec::new();

    // ERROR checks - structural breaks
    check_node_count(selkie, reference, &mut issues);
    check_edge_count(selkie, reference, &mut issues);
    check_missing_labels(selkie, reference, &mut issues);

    // WARNING checks - significant differences
    check_dimensions(selkie, reference, config, &mut issues);
    check_shape_counts(selkie, reference, &mut issues);
    check_z_order(selkie, reference, &mut issues);

    // INFO checks - acceptable variations
    check_extra_labels(selkie, reference, &mut issues);
    check_markers(selkie, reference, &mut issues);

    issues
}

/// Check node count - ERROR if mismatch
fn check_node_count(selkie: &SvgStructure, reference: &SvgStructure, issues: &mut Vec<Issue>) {
    if selkie.node_count != reference.node_count {
        issues.push(
            Issue::error(
                "node_count",
                format!(
                    "Node count mismatch: expected {}, got {}",
                    reference.node_count, selkie.node_count
                ),
            )
            .with_values(
                reference.node_count.to_string(),
                selkie.node_count.to_string(),
            ),
        );
    }
}

/// Check edge count - ERROR if mismatch
fn check_edge_count(selkie: &SvgStructure, reference: &SvgStructure, issues: &mut Vec<Issue>) {
    if selkie.edge_count != reference.edge_count {
        issues.push(
            Issue::error(
                "edge_count",
                format!(
                    "Edge count mismatch: expected {}, got {}",
                    reference.edge_count, selkie.edge_count
                ),
            )
            .with_values(
                reference.edge_count.to_string(),
                selkie.edge_count.to_string(),
            ),
        );
    }
}

/// Check for missing labels - ERROR if labels from reference are missing
fn check_missing_labels(selkie: &SvgStructure, reference: &SvgStructure, issues: &mut Vec<Issue>) {
    let selkie_labels: HashSet<_> = selkie.labels.iter().collect();
    let reference_labels: HashSet<_> = reference.labels.iter().collect();

    let missing: Vec<_> = reference_labels
        .difference(&selkie_labels)
        .cloned()
        .collect();

    if !missing.is_empty() {
        issues.push(
            Issue::error("labels_missing", format!("Missing labels: {:?}", missing)).with_values(
                format!("{:?}", reference.labels),
                format!("{:?}", selkie.labels),
            ),
        );
    }
}

/// Check for extra labels - INFO (acceptable variation)
fn check_extra_labels(selkie: &SvgStructure, reference: &SvgStructure, issues: &mut Vec<Issue>) {
    let selkie_labels: HashSet<_> = selkie.labels.iter().collect();
    let reference_labels: HashSet<_> = reference.labels.iter().collect();

    let extra: Vec<_> = selkie_labels
        .difference(&reference_labels)
        .cloned()
        .collect();

    if !extra.is_empty() {
        issues.push(Issue::info(
            "labels_extra",
            format!("Extra labels in selkie: {:?}", extra),
        ));
    }
}

/// Check dimensions - WARNING if >20% off, INFO if >5% off
fn check_dimensions(
    selkie: &SvgStructure,
    reference: &SvgStructure,
    config: &CheckConfig,
    issues: &mut Vec<Issue>,
) {
    // Width check
    let width_diff = if reference.width > 0.0 {
        (selkie.width - reference.width).abs() / reference.width
    } else {
        0.0
    };

    if width_diff > config.dimension_warning_threshold {
        issues.push(
            Issue::warning(
                "dimensions",
                format!(
                    "Width differs by {:.0}%: expected {:.0}, got {:.0}",
                    width_diff * 100.0,
                    reference.width,
                    selkie.width
                ),
            )
            .with_values(
                format!("{:.0}", reference.width),
                format!("{:.0}", selkie.width),
            ),
        );
    } else if width_diff > config.dimension_info_threshold {
        issues.push(Issue::info(
            "dimensions",
            format!(
                "Width differs by {:.0}%: expected {:.0}, got {:.0}",
                width_diff * 100.0,
                reference.width,
                selkie.width
            ),
        ));
    }

    // Height check
    let height_diff = if reference.height > 0.0 {
        (selkie.height - reference.height).abs() / reference.height
    } else {
        0.0
    };

    if height_diff > config.dimension_warning_threshold {
        issues.push(
            Issue::warning(
                "dimensions",
                format!(
                    "Height differs by {:.0}%: expected {:.0}, got {:.0}",
                    height_diff * 100.0,
                    reference.height,
                    selkie.height
                ),
            )
            .with_values(
                format!("{:.0}", reference.height),
                format!("{:.0}", selkie.height),
            ),
        );
    } else if height_diff > config.dimension_info_threshold {
        issues.push(Issue::info(
            "dimensions",
            format!(
                "Height differs by {:.0}%: expected {:.0}, got {:.0}",
                height_diff * 100.0,
                reference.height,
                selkie.height
            ),
        ));
    }
}

/// Check shape counts - WARNING if significantly different
fn check_shape_counts(selkie: &SvgStructure, reference: &SvgStructure, issues: &mut Vec<Issue>) {
    let shape_checks = [
        ("rect", selkie.shapes.rect, reference.shapes.rect),
        ("circle", selkie.shapes.circle, reference.shapes.circle),
        ("ellipse", selkie.shapes.ellipse, reference.shapes.ellipse),
        ("polygon", selkie.shapes.polygon, reference.shapes.polygon),
        ("path", selkie.shapes.path, reference.shapes.path),
        ("line", selkie.shapes.line, reference.shapes.line),
        (
            "polyline",
            selkie.shapes.polyline,
            reference.shapes.polyline,
        ),
    ];

    for (name, selkie_count, ref_count) in shape_checks {
        if selkie_count != ref_count {
            let diff_pct = if ref_count > 0 {
                ((selkie_count as f64 - ref_count as f64) / ref_count as f64 * 100.0).abs()
            } else if selkie_count > 0 {
                100.0
            } else {
                0.0
            };

            // Only report if >20% difference to avoid noise
            if diff_pct > 20.0 {
                issues.push(
                    Issue::warning(
                        "shapes",
                        format!(
                            "{} count differs: expected {}, got {} ({:.0}% diff)",
                            name, ref_count, selkie_count, diff_pct
                        ),
                    )
                    .with_values(ref_count.to_string(), selkie_count.to_string()),
                );
            }
        }
    }
}

/// Check marker count - INFO if different
fn check_markers(selkie: &SvgStructure, reference: &SvgStructure, issues: &mut Vec<Issue>) {
    if selkie.marker_count != reference.marker_count {
        issues.push(Issue::info(
            "markers",
            format!(
                "Marker count differs: expected {}, got {}",
                reference.marker_count, selkie.marker_count
            ),
        ));
    }
}

/// Check z-order (element rendering order) - WARNING if text may be obscured
fn check_z_order(selkie: &SvgStructure, reference: &SvgStructure, issues: &mut Vec<Issue>) {
    // Check if selkie has text rendered before shapes when reference doesn't
    // This would cause text to be hidden behind shapes
    if selkie.z_order.text_before_shapes > reference.z_order.text_before_shapes {
        let diff = selkie.z_order.text_before_shapes - reference.z_order.text_before_shapes;
        let mut msg = format!(
            "Z-order issue: {} text element(s) rendered before shapes (may be obscured)",
            diff
        );

        if !selkie.z_order.potentially_obscured_labels.is_empty() {
            msg.push_str(&format!(
                ". Potentially affected labels: {:?}",
                selkie.z_order.potentially_obscured_labels
            ));
        }

        issues.push(Issue::warning("z_order", msg).with_values(
            format!(
                "text_before_shapes: {}",
                reference.z_order.text_before_shapes
            ),
            format!("text_before_shapes: {}", selkie.z_order.text_before_shapes),
        ));
    }

    // Also warn if the overall text/shape ordering pattern differs significantly
    let selkie_ratio = if selkie.z_order.text_after_shapes + selkie.z_order.text_before_shapes > 0 {
        selkie.z_order.text_after_shapes as f64
            / (selkie.z_order.text_after_shapes + selkie.z_order.text_before_shapes) as f64
    } else {
        1.0
    };

    let ref_ratio = if reference.z_order.text_after_shapes + reference.z_order.text_before_shapes
        > 0
    {
        reference.z_order.text_after_shapes as f64
            / (reference.z_order.text_after_shapes + reference.z_order.text_before_shapes) as f64
    } else {
        1.0
    };

    // If reference has >80% text-after-shapes but selkie has <50%, that's a significant difference
    if ref_ratio > 0.8 && selkie_ratio < 0.5 {
        issues.push(Issue::warning(
            "z_order_pattern",
            format!(
                "Z-order pattern differs: reference has {:.0}% text after shapes, selkie has {:.0}%",
                ref_ratio * 100.0,
                selkie_ratio * 100.0
            ),
        ));
    }
}

/// Calculate structural similarity score (0-1)
pub fn calculate_similarity(selkie: &SvgStructure, reference: &SvgStructure) -> f64 {
    let mut score_parts: Vec<f64> = Vec::new();

    // Node count similarity
    if reference.node_count > 0 || selkie.node_count > 0 {
        let min = selkie.node_count.min(reference.node_count) as f64;
        let max = selkie.node_count.max(reference.node_count) as f64;
        score_parts.push(if max > 0.0 { min / max } else { 1.0 });
    }

    // Edge count similarity
    if reference.edge_count > 0 || selkie.edge_count > 0 {
        let min = selkie.edge_count.min(reference.edge_count) as f64;
        let max = selkie.edge_count.max(reference.edge_count) as f64;
        score_parts.push(if max > 0.0 { min / max } else { 1.0 });
    }

    // Label similarity
    let selkie_labels: HashSet<_> = selkie.labels.iter().collect();
    let reference_labels: HashSet<_> = reference.labels.iter().collect();
    let common = selkie_labels.intersection(&reference_labels).count() as f64;
    let total = selkie_labels.len().max(reference_labels.len()) as f64;
    if total > 0.0 {
        score_parts.push(common / total);
    }

    // Dimension similarity
    if reference.width > 0.0 {
        let width_ratio = selkie.width.min(reference.width) / selkie.width.max(reference.width);
        score_parts.push(width_ratio);
    }
    if reference.height > 0.0 {
        let height_ratio =
            selkie.height.min(reference.height) / selkie.height.max(reference.height);
        score_parts.push(height_ratio);
    }

    // Calculate average
    if score_parts.is_empty() {
        1.0
    } else {
        score_parts.iter().sum::<f64>() / score_parts.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::Level;
    use crate::render::svg::structure::{ShapeCounts, ZOrderAnalysis};

    fn make_structure(nodes: usize, edges: usize, labels: Vec<&str>) -> SvgStructure {
        SvgStructure {
            width: 400.0,
            height: 300.0,
            node_count: nodes,
            edge_count: edges,
            labels: labels.into_iter().map(String::from).collect(),
            shapes: ShapeCounts::default(),
            marker_count: 0,
            has_defs: true,
            has_style: true,
            z_order: ZOrderAnalysis::default(),
        }
    }

    #[test]
    fn test_identical_structures() {
        let s1 = make_structure(3, 2, vec!["A", "B", "C"]);
        let s2 = make_structure(3, 2, vec!["A", "B", "C"]);

        let issues = check_structure(&s1, &s2, &CheckConfig::default());
        let errors: Vec<_> = issues.iter().filter(|i| i.level == Level::Error).collect();
        assert!(
            errors.is_empty(),
            "Should have no errors for identical structures"
        );
    }

    #[test]
    fn test_node_count_mismatch() {
        let selkie = make_structure(3, 2, vec!["A", "B", "C"]);
        let reference = make_structure(4, 2, vec!["A", "B", "C", "D"]);

        let issues = check_structure(&selkie, &reference, &CheckConfig::default());
        let errors: Vec<_> = issues.iter().filter(|i| i.level == Level::Error).collect();
        assert!(
            !errors.is_empty(),
            "Should have error for node count mismatch"
        );
    }

    #[test]
    fn test_missing_labels() {
        let selkie = make_structure(3, 2, vec!["A", "B"]);
        let reference = make_structure(3, 2, vec!["A", "B", "C"]);

        let issues = check_structure(&selkie, &reference, &CheckConfig::default());
        let has_missing_label_error = issues
            .iter()
            .any(|i| i.level == Level::Error && i.check == "labels_missing");
        assert!(
            has_missing_label_error,
            "Should have error for missing labels"
        );
    }

    #[test]
    fn test_similarity_identical() {
        let s1 = make_structure(3, 2, vec!["A", "B", "C"]);
        let s2 = make_structure(3, 2, vec!["A", "B", "C"]);

        let sim = calculate_similarity(&s1, &s2);
        assert!(
            (sim - 1.0).abs() < 0.01,
            "Identical structures should have 1.0 similarity"
        );
    }

    #[test]
    fn test_similarity_different() {
        let s1 = make_structure(3, 2, vec!["A", "B", "C"]);
        let s2 = make_structure(6, 4, vec!["A", "B", "C", "D", "E", "F"]);

        let sim = calculate_similarity(&s1, &s2);
        assert!(
            sim < 0.8,
            "Different structures should have lower similarity"
        );
    }
}
