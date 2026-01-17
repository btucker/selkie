//! Structural checks for comparing SVG outputs.
//!
//! This module defines the 3-level check system:
//! - Error: Structural breaks (node/edge count mismatch, missing labels)
//! - Warning: Significant differences (dimensions >20% off, shape counts differ)
//! - Info: Acceptable variations (styling, minor dimension differences)

use super::Issue;
use crate::render::svg::structure::{EdgeGeometry, NodeBounds};
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
    check_stroke_widths(selkie, reference, &mut issues);
    check_edge_attachments(selkie, reference, &mut issues);
    check_edge_node_connectivity(selkie, reference, &mut issues);
    check_font_styles(selkie, reference, &mut issues);

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

/// Check stroke-width differences - WARNING if significantly different
fn check_stroke_widths(selkie: &SvgStructure, reference: &SvgStructure, issues: &mut Vec<Issue>) {
    let selkie_stroke = &selkie.stroke_analysis;
    let ref_stroke = &reference.stroke_analysis;

    // Check rect (border) stroke width differences
    if ref_stroke.avg_rect_stroke > 0.0 && selkie_stroke.avg_rect_stroke > 0.0 {
        let diff = (selkie_stroke.avg_rect_stroke - ref_stroke.avg_rect_stroke).abs();
        let pct_diff = diff / ref_stroke.avg_rect_stroke * 100.0;

        // Warn if >30% difference in border stroke width
        if pct_diff > 30.0 {
            issues.push(
                Issue::warning(
                    "stroke_width",
                    format!(
                        "Border stroke-width differs: expected {:.1}, got {:.1} ({:.0}% diff)",
                        ref_stroke.avg_rect_stroke, selkie_stroke.avg_rect_stroke, pct_diff
                    ),
                )
                .with_values(
                    format!("{:.1}", ref_stroke.avg_rect_stroke),
                    format!("{:.1}", selkie_stroke.avg_rect_stroke),
                ),
            );
        }
    }

    // Check path (edge) stroke width differences
    if ref_stroke.avg_path_stroke > 0.0 && selkie_stroke.avg_path_stroke > 0.0 {
        let diff = (selkie_stroke.avg_path_stroke - ref_stroke.avg_path_stroke).abs();
        let pct_diff = diff / ref_stroke.avg_path_stroke * 100.0;

        // Warn if >30% difference in edge stroke width
        if pct_diff > 30.0 {
            issues.push(
                Issue::warning(
                    "stroke_width",
                    format!(
                        "Edge stroke-width differs: expected {:.1}, got {:.1} ({:.0}% diff)",
                        ref_stroke.avg_path_stroke, selkie_stroke.avg_path_stroke, pct_diff
                    ),
                )
                .with_values(
                    format!("{:.1}", ref_stroke.avg_path_stroke),
                    format!("{:.1}", selkie_stroke.avg_path_stroke),
                ),
            );
        }
    }
}

/// Check edge attachment points - WARNING if edges attach differently
fn check_edge_attachments(
    selkie: &SvgStructure,
    reference: &SvgStructure,
    issues: &mut Vec<Issue>,
) {
    let selkie_geo = &selkie.edge_geometry;
    let ref_geo = &reference.edge_geometry;

    // Only compare if both have edges
    if selkie_geo.edge_details.is_empty() && ref_geo.edge_details.is_empty() {
        return;
    }

    // Build a summary of edge attachments for clear comparison
    let mut selkie_summary = Vec::new();
    let mut ref_summary = Vec::new();

    for (i, edge) in selkie_geo.edge_details.iter().enumerate() {
        let start_desc = if edge.start_center_offset.abs() < 5.0 {
            format!("{} (centered)", edge.start_edge)
        } else {
            format!(
                "{} (offset {:.0}px)",
                edge.start_edge, edge.start_center_offset
            )
        };
        let end_desc = if edge.end_center_offset.abs() < 5.0 {
            format!("{} (centered)", edge.end_edge)
        } else {
            format!("{} (offset {:.0}px)", edge.end_edge, edge.end_center_offset)
        };

        selkie_summary.push(format!(
            "Edge {}: {} → {}",
            i + 1,
            edge.start_node
                .as_ref()
                .map(|n| format!("{}.{}", n, start_desc))
                .unwrap_or_else(|| format!("({:.0},{:.0})", edge.start.0, edge.start.1)),
            edge.end_node
                .as_ref()
                .map(|n| format!("{}.{}", n, end_desc))
                .unwrap_or_else(|| format!("({:.0},{:.0})", edge.end.0, edge.end.1)),
        ));
    }

    for (i, edge) in ref_geo.edge_details.iter().enumerate() {
        let start_desc = if edge.start_center_offset.abs() < 5.0 {
            format!("{} (centered)", edge.start_edge)
        } else {
            format!(
                "{} (offset {:.0}px)",
                edge.start_edge, edge.start_center_offset
            )
        };
        let end_desc = if edge.end_center_offset.abs() < 5.0 {
            format!("{} (centered)", edge.end_edge)
        } else {
            format!("{} (offset {:.0}px)", edge.end_edge, edge.end_center_offset)
        };

        ref_summary.push(format!(
            "Edge {}: {} → {}",
            i + 1,
            edge.start_node
                .as_ref()
                .map(|n| format!("{}.{}", n, start_desc))
                .unwrap_or_else(|| format!("({:.0},{:.0})", edge.start.0, edge.start.1)),
            edge.end_node
                .as_ref()
                .map(|n| format!("{}.{}", n, end_desc))
                .unwrap_or_else(|| format!("({:.0},{:.0})", edge.end.0, edge.end.1)),
        ));
    }

    // Analyze edge differences for clear AI feedback
    let has_edges = !selkie_geo.edge_endpoints.is_empty() || !ref_geo.edge_endpoints.is_empty();

    if has_edges {
        // Check for edge count mismatch
        let selkie_count = selkie_geo.edge_endpoints.len();
        let ref_count = ref_geo.edge_endpoints.len();

        if selkie_count != ref_count {
            issues.push(
                Issue::warning(
                    "edge_count",
                    format!(
                        "Edge count differs: expected {}, got {}",
                        ref_count, selkie_count
                    ),
                )
                .with_values(ref_count.to_string(), selkie_count.to_string()),
            );
        }

        // Build concise edge comparison
        let min_count = selkie_count.min(ref_count);
        let mut edge_diffs = Vec::new();

        for i in 0..min_count {
            let (sx1, sy1, sx2, sy2) = selkie_geo.edge_endpoints[i];
            let (rx1, ry1, rx2, ry2) = ref_geo.edge_endpoints[i];

            // Check if edge paths differ significantly (>10px)
            let start_diff = ((sx1 - rx1).powi(2) + (sy1 - ry1).powi(2)).sqrt();
            let end_diff = ((sx2 - rx2).powi(2) + (sy2 - ry2).powi(2)).sqrt();

            if start_diff > 10.0 || end_diff > 10.0 {
                // Classify the edge direction
                let selkie_dir = classify_edge_direction((sx1, sy1), (sx2, sy2));
                let ref_dir = classify_edge_direction((rx1, ry1), (rx2, ry2));

                edge_diffs.push(format!(
                    "Edge {}: selkie={} ref={} (start diff={:.0}px, end diff={:.0}px)",
                    i + 1,
                    selkie_dir,
                    ref_dir,
                    start_diff,
                    end_diff
                ));
            }
        }

        if !edge_diffs.is_empty() {
            let message = format!("EDGE POSITION DIFFERENCES:\n  {}", edge_diffs.join("\n  "));
            issues.push(Issue::warning("edge_positions", message));
        }
    }

    // Check for attachment SIDE mismatches (e.g., selkie attaches to "top" but reference to "left")
    // This is a critical issue because it means edges are connecting to the wrong sides of entities
    let selkie_details = &selkie_geo.edge_details;
    let ref_details = &ref_geo.edge_details;
    let selkie_endpoints = &selkie_geo.edge_endpoints;
    let ref_endpoints = &ref_geo.edge_endpoints;

    // Use whichever data source is available
    let edge_count = selkie_details
        .len()
        .min(ref_details.len())
        .max(selkie_endpoints.len().min(ref_endpoints.len()));

    let mut side_mismatches = Vec::new();
    for i in 0..edge_count {
        // Get selkie START attachment side (from edge_details or inferred from endpoints)
        let selkie_start_side = if i < selkie_details.len() {
            normalize_edge_side(&selkie_details[i].start_edge)
        } else if i < selkie_endpoints.len() {
            let (sx, sy, ex, ey) = selkie_endpoints[i];
            infer_start_attachment_side((sx, sy), (ex, ey))
        } else {
            "unknown".to_string()
        };

        // Get reference START attachment side (from edge_details or inferred from endpoints)
        // Use initial direction (second point) for accurate inference on curved paths
        let ref_initial_dirs = &ref_geo.edge_initial_directions;
        let ref_start_side = if i < ref_details.len() && ref_details[i].start_edge != "none" {
            normalize_edge_side(&ref_details[i].start_edge)
        } else if i < ref_endpoints.len() {
            let (sx, sy, ex, ey) = ref_endpoints[i];
            let second_point = ref_initial_dirs.get(i).copied().flatten();
            infer_start_attachment_with_direction((sx, sy), second_point, (ex, ey))
        } else {
            "unknown".to_string()
        };

        // Get selkie END attachment side (from edge_details or inferred from endpoints)
        let selkie_end_side = if i < selkie_details.len() {
            normalize_edge_side(&selkie_details[i].end_edge)
        } else if i < selkie_endpoints.len() {
            let (sx, sy, ex, ey) = selkie_endpoints[i];
            infer_end_attachment_side((sx, sy), (ex, ey))
        } else {
            "unknown".to_string()
        };

        // Get reference END attachment side (from edge_details or inferred from endpoints)
        let ref_end_side = if i < ref_details.len() && ref_details[i].end_edge != "none" {
            normalize_edge_side(&ref_details[i].end_edge)
        } else if i < ref_endpoints.len() {
            let (sx, sy, ex, ey) = ref_endpoints[i];
            infer_end_attachment_side((sx, sy), (ex, ey))
        } else {
            "unknown".to_string()
        };

        // Check if START attachment sides are categorically different
        // top/bottom are "vertical", left/right are "horizontal"
        let selkie_start_is_vertical = matches!(selkie_start_side.as_str(), "top" | "bottom");
        let ref_start_is_vertical = matches!(ref_start_side.as_str(), "top" | "bottom");

        if selkie_start_side != "unknown"
            && ref_start_side != "unknown"
            && selkie_start_is_vertical != ref_start_is_vertical
        {
            side_mismatches.push(format!(
                "Edge {} start: leaves from {} in selkie but {} in reference",
                i + 1,
                selkie_start_side,
                ref_start_side
            ));
        }

        // Check if END attachment sides are categorically different
        let selkie_end_is_vertical = matches!(selkie_end_side.as_str(), "top" | "bottom");
        let ref_end_is_vertical = matches!(ref_end_side.as_str(), "top" | "bottom");

        if selkie_end_side != "unknown"
            && ref_end_side != "unknown"
            && selkie_end_is_vertical != ref_end_is_vertical
        {
            side_mismatches.push(format!(
                "Edge {} end: attaches to {} in selkie but {} in reference",
                i + 1,
                selkie_end_side,
                ref_end_side
            ));
        }
    }

    if !side_mismatches.is_empty() {
        let message = format!(
            "ATTACHMENT SIDE MISMATCHES (edges connect to wrong entity sides):\n  {}",
            side_mismatches.join("\n  ")
        );
        // This is an ERROR because attaching to the wrong side is a significant visual bug
        // (e.g., crow's feet pointing at top instead of left/right)
        issues.push(Issue::error("edge_attachment_sides", message));
    }

    // Compare edges if we have detailed info
    if !selkie_summary.is_empty() || !ref_summary.is_empty() {
        // Output detailed edge attachment info for AI analysis
        if !selkie_summary.is_empty() || !ref_summary.is_empty() {
            let message = format!(
                "EDGE ATTACHMENTS:\n  Reference:\n    {}\n  Selkie:\n    {}",
                if ref_summary.is_empty() {
                    "(none)".to_string()
                } else {
                    ref_summary.join("\n    ")
                },
                if selkie_summary.is_empty() {
                    "(none)".to_string()
                } else {
                    selkie_summary.join("\n    ")
                }
            );
            issues.push(Issue::info("edge_details", message));
        }
    }

    // Also check overall pattern
    let selkie_total = selkie_geo.vertical_attachments + selkie_geo.horizontal_attachments;
    let ref_total = ref_geo.vertical_attachments + ref_geo.horizontal_attachments;

    if selkie_total > 0 && ref_total > 0 {
        let selkie_vert_ratio = selkie_geo.vertical_attachments as f64 / selkie_total as f64;
        let ref_vert_ratio = ref_geo.vertical_attachments as f64 / ref_total as f64;
        let ratio_diff = (selkie_vert_ratio - ref_vert_ratio).abs();

        if ratio_diff > 0.3 {
            let selkie_pattern = if selkie_vert_ratio > 0.6 {
                "mostly top/bottom"
            } else if selkie_vert_ratio < 0.4 {
                "mostly sides"
            } else {
                "mixed"
            };
            let ref_pattern = if ref_vert_ratio > 0.6 {
                "mostly top/bottom"
            } else if ref_vert_ratio < 0.4 {
                "mostly sides"
            } else {
                "mixed"
            };

            if selkie_pattern != ref_pattern {
                issues.push(
                    Issue::warning(
                        "edge_attachment_pattern",
                        format!(
                            "Edge attachment pattern differs: reference is {}, selkie is {}",
                            ref_pattern, selkie_pattern
                        ),
                    )
                    .with_values(
                        format!(
                            "vertical: {}, horizontal: {}",
                            ref_geo.vertical_attachments, ref_geo.horizontal_attachments
                        ),
                        format!(
                            "vertical: {}, horizontal: {}",
                            selkie_geo.vertical_attachments, selkie_geo.horizontal_attachments
                        ),
                    ),
                );
            }
        }
    }
}

/// Classify edge direction based on start and end points
fn classify_edge_direction(start: (f64, f64), end: (f64, f64)) -> &'static str {
    let dx = (end.0 - start.0).abs();
    let dy = (end.1 - start.1).abs();

    if dx < 10.0 && dy > 10.0 {
        "vertical"
    } else if dy < 10.0 && dx > 10.0 {
        "horizontal"
    } else if dx > 10.0 && dy > 10.0 {
        "diagonal"
    } else {
        "point"
    }
}

/// Normalize edge side names for comparison
/// Handles variations like "none" -> "unknown"
fn normalize_edge_side(side: &str) -> String {
    match side.to_lowercase().as_str() {
        "top" => "top".to_string(),
        "bottom" => "bottom".to_string(),
        "left" => "left".to_string(),
        "right" => "right".to_string(),
        "none" | "" => "unknown".to_string(),
        other => other.to_string(),
    }
}

/// Infer END attachment side from edge endpoint coordinates
/// This is used when node_bounds aren't available (e.g., for reference SVGs)
/// Returns the likely attachment side based on the edge direction at the endpoint
fn infer_end_attachment_side(start: (f64, f64), end: (f64, f64)) -> String {
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;

    // Determine the dominant direction at the endpoint
    // If the edge is mostly vertical, it attaches to top/bottom
    // If mostly horizontal, it attaches to left/right
    let dx_abs = dx.abs();
    let dy_abs = dy.abs();

    if dy_abs > dx_abs * 1.5 {
        // Mostly vertical - attaching to top or bottom
        if dy > 0.0 {
            "top".to_string() // coming from above, attaching to top
        } else {
            "bottom".to_string() // coming from below, attaching to bottom
        }
    } else if dx_abs > dy_abs * 1.5 {
        // Mostly horizontal - attaching to left or right
        if dx > 0.0 {
            "left".to_string() // coming from left, attaching to left side
        } else {
            "right".to_string() // coming from right, attaching to right side
        }
    } else {
        // Diagonal - use the larger component
        if dy_abs > dx_abs {
            if dy > 0.0 {
                "top".to_string()
            } else {
                "bottom".to_string()
            }
        } else if dx > 0.0 {
            "left".to_string()
        } else {
            "right".to_string()
        }
    }
}

/// Infer START attachment side from edge endpoint coordinates
/// This is the opposite of infer_end_attachment_side - determines which side
/// the edge leaves FROM based on its direction
fn infer_start_attachment_side(start: (f64, f64), end: (f64, f64)) -> String {
    infer_attachment_direction(start, end)
}

/// Infer the attachment side based on direction from point A to point B.
/// For start attachment: A=start, B=second_point or end
/// For end attachment: A=second_last_point or start, B=end
fn infer_attachment_direction(from: (f64, f64), to: (f64, f64)) -> String {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;

    let dx_abs = dx.abs();
    let dy_abs = dy.abs();

    if dy_abs > dx_abs * 1.5 {
        // Mostly vertical
        if dy > 0.0 {
            "bottom".to_string() // going down, leaving from bottom
        } else {
            "top".to_string() // going up, leaving from top
        }
    } else if dx_abs > dy_abs * 1.5 {
        // Mostly horizontal
        if dx > 0.0 {
            "right".to_string() // going right, leaving from right side
        } else {
            "left".to_string() // going left, leaving from left side
        }
    } else {
        // Diagonal - use the larger component
        if dy_abs > dx_abs {
            if dy > 0.0 {
                "bottom".to_string()
            } else {
                "top".to_string()
            }
        } else if dx > 0.0 {
            "right".to_string()
        } else {
            "left".to_string()
        }
    }
}

/// Infer start attachment side using the initial direction (second point) if available.
/// This is crucial for curved paths where the overall direction differs from the initial tangent.
fn infer_start_attachment_with_direction(
    start: (f64, f64),
    second_point: Option<(f64, f64)>,
    end: (f64, f64),
) -> String {
    // If we have a second point (initial direction), use it for accurate inference
    if let Some(sp) = second_point {
        infer_attachment_direction(start, sp)
    } else {
        // Fall back to using overall direction
        infer_attachment_direction(start, end)
    }
}

/// Check font styles (size, weight) - WARNING if significantly different
fn check_font_styles(selkie: &SvgStructure, reference: &SvgStructure, issues: &mut Vec<Issue>) {
    let selkie_fonts = &selkie.font_analysis;
    let ref_fonts = &reference.font_analysis;

    // Helper to parse font size string to numeric value
    fn parse_font_size(s: &str) -> Option<f64> {
        s.trim_end_matches("px").parse().ok()
    }

    // Collect all font sizes as numeric values
    let selkie_all_sizes: Vec<f64> = selkie_fonts
        .font_sizes
        .iter()
        .filter_map(|fs| parse_font_size(&fs.value))
        .collect();

    let ref_all_sizes: Vec<f64> = ref_fonts
        .font_sizes
        .iter()
        .filter_map(|fs| parse_font_size(&fs.value))
        .collect();

    // Compare max font sizes (typically entity names / headers)
    if !selkie_all_sizes.is_empty() && !ref_all_sizes.is_empty() {
        let selkie_max = selkie_all_sizes.iter().cloned().fold(0.0, f64::max);
        let ref_max = ref_all_sizes.iter().cloned().fold(0.0, f64::max);

        // More than 2px difference in max font size is significant
        if (ref_max - selkie_max).abs() > 2.0 {
            issues.push(
                Issue::warning(
                    "font_size",
                    format!(
                        "Max font size differs: reference uses {}px, selkie uses {}px ({}px smaller)",
                        ref_max, selkie_max, ref_max - selkie_max
                    ),
                )
                .with_values(format!("{}px", ref_max), format!("{}px", selkie_max)),
            );
        }

        // Compare min font sizes (typically attribute text)
        let selkie_min = selkie_all_sizes.iter().cloned().fold(f64::MAX, f64::min);
        let ref_min = ref_all_sizes.iter().cloned().fold(f64::MAX, f64::min);

        if (ref_min - selkie_min).abs() > 2.0 {
            issues.push(
                Issue::warning(
                    "font_size",
                    format!(
                        "Min font size differs: reference uses {}px, selkie uses {}px ({}px smaller)",
                        ref_min, selkie_min, ref_min - selkie_min
                    ),
                )
                .with_values(format!("{}px", ref_min), format!("{}px", selkie_min)),
            );
        }
    }

    // Build maps of context -> sizes for detailed comparison
    let selkie_sizes: std::collections::HashMap<String, Vec<String>> = selkie_fonts
        .font_sizes
        .iter()
        .fold(std::collections::HashMap::new(), |mut acc, fs| {
            acc.entry(fs.context.clone())
                .or_default()
                .push(fs.value.clone());
            acc
        });

    let ref_sizes: std::collections::HashMap<String, Vec<String>> = ref_fonts
        .font_sizes
        .iter()
        .fold(std::collections::HashMap::new(), |mut acc, fs| {
            acc.entry(fs.context.clone())
                .or_default()
                .push(fs.value.clone());
            acc
        });

    // Check for context-specific font size mismatches
    for (context, ref_values) in &ref_sizes {
        if let Some(selkie_values) = selkie_sizes.get(context) {
            // Check if any values differ significantly
            for ref_val in ref_values {
                let ref_num: Option<f64> = parse_font_size(ref_val);
                let mut found_match = false;

                for selkie_val in selkie_values {
                    let selkie_num: Option<f64> = parse_font_size(selkie_val);

                    if let (Some(r), Some(s)) = (ref_num, selkie_num) {
                        // Allow 2px tolerance
                        if (r - s).abs() <= 2.0 {
                            found_match = true;
                            break;
                        }
                    } else if ref_val == selkie_val {
                        found_match = true;
                        break;
                    }
                }

                if !found_match && ref_num.is_some() {
                    issues.push(
                        Issue::warning(
                            "font_size",
                            format!(
                                "Font size mismatch for '{}': expected {}, got {:?}",
                                context, ref_val, selkie_values
                            ),
                        )
                        .with_values(ref_val.clone(), selkie_values.join(", ")),
                    );
                    break; // Only report once per context
                }
            }
        }
    }

    // Build maps of context -> weights for comparison
    let selkie_weights: std::collections::HashMap<String, Vec<String>> = selkie_fonts
        .font_weights
        .iter()
        .fold(std::collections::HashMap::new(), |mut acc, fs| {
            acc.entry(fs.context.clone())
                .or_default()
                .push(fs.value.clone());
            acc
        });

    let ref_weights: std::collections::HashMap<String, Vec<String>> = ref_fonts
        .font_weights
        .iter()
        .fold(std::collections::HashMap::new(), |mut acc, fs| {
            acc.entry(fs.context.clone())
                .or_default()
                .push(fs.value.clone());
            acc
        });

    // Check for missing font weights
    for (context, ref_values) in &ref_weights {
        if let Some(selkie_values) = selkie_weights.get(context) {
            for ref_val in ref_values {
                if !selkie_values.contains(ref_val) {
                    // Normalize weight comparisons (e.g., "bold" = "700")
                    let ref_normalized = normalize_font_weight(ref_val);
                    let selkie_normalized: Vec<String> = selkie_values
                        .iter()
                        .map(|v| normalize_font_weight(v))
                        .collect();

                    if !selkie_normalized.contains(&ref_normalized) {
                        issues.push(
                            Issue::warning(
                                "font_weight",
                                format!(
                                    "Font weight mismatch for '{}': expected {}, got {:?}",
                                    context, ref_val, selkie_values
                                ),
                            )
                            .with_values(ref_val.clone(), selkie_values.join(", ")),
                        );
                        break; // Only report once per context
                    }
                }
            }
        }
    }
}

/// Normalize font weight values (e.g., "bold" -> "700")
fn normalize_font_weight(weight: &str) -> String {
    match weight.trim().to_lowercase().as_str() {
        "normal" => "400".to_string(),
        "bold" => "700".to_string(),
        "lighter" => "lighter".to_string(),
        "bolder" => "bolder".to_string(),
        other => other.to_string(),
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

/// Check if edge endpoints touch node boundaries - ERROR if selkie has disconnected edges
/// that the reference doesn't have.
///
/// This detects a critical rendering bug where crow's feet or edge endpoints
/// don't connect to their target nodes, making the diagram incorrect.
fn check_edge_node_connectivity(
    selkie: &SvgStructure,
    reference: &SvgStructure,
    issues: &mut Vec<Issue>,
) {
    let selkie_geo = &selkie.edge_geometry;
    let ref_geo = &reference.edge_geometry;

    // Need edges and nodes in selkie to check connectivity
    if selkie_geo.edge_endpoints.is_empty() || selkie_geo.node_bounds.is_empty() {
        return;
    }

    // Tolerance for "touching" - edges should be within this distance of a node boundary
    let tolerance = 5.0;

    // Count disconnected edges in selkie
    let selkie_disconnected = count_disconnected_edges(selkie_geo, tolerance);

    // Count disconnected edges in reference (if data available)
    let ref_disconnected = if ref_geo.node_bounds.is_empty() {
        0 // Can't check reference, assume it's fine
    } else {
        count_disconnected_edges(ref_geo, tolerance)
    };

    // Only report if selkie has MORE disconnected edges than reference
    // (reference may also have some due to SVG structure parsing limitations)
    if selkie_disconnected > ref_disconnected {
        let mut messages = Vec::new();

        for (i, &(start_x, start_y, end_x, end_y)) in selkie_geo.edge_endpoints.iter().enumerate() {
            let start_touches =
                point_touches_any_node(start_x, start_y, &selkie_geo.node_bounds, tolerance);
            let end_touches =
                point_touches_any_node(end_x, end_y, &selkie_geo.node_bounds, tolerance);

            if !start_touches {
                messages.push(format!(
                    "Edge {} start ({:.0},{:.0}) doesn't touch any node",
                    i + 1,
                    start_x,
                    start_y
                ));
            }
            if !end_touches {
                messages.push(format!(
                    "Edge {} end ({:.0},{:.0}) doesn't touch any node",
                    i + 1,
                    end_x,
                    end_y
                ));
            }
        }

        if !messages.is_empty() {
            issues.push(Issue::error(
                "edge_connectivity",
                format!(
                    "DISCONNECTED EDGES (endpoints not touching nodes):\n  {}",
                    messages.join("\n  ")
                ),
            ));
        }
    }
}

/// Count how many edge endpoints don't touch any node boundary
fn count_disconnected_edges(geometry: &EdgeGeometry, tolerance: f64) -> usize {
    let mut count = 0;
    for &(start_x, start_y, end_x, end_y) in &geometry.edge_endpoints {
        if !point_touches_any_node(start_x, start_y, &geometry.node_bounds, tolerance) {
            count += 1;
        }
        if !point_touches_any_node(end_x, end_y, &geometry.node_bounds, tolerance) {
            count += 1;
        }
    }
    count
}

/// Check if a point is within tolerance of any node's boundary
fn point_touches_any_node(x: f64, y: f64, nodes: &[NodeBounds], tolerance: f64) -> bool {
    for node in nodes {
        if point_touches_node_boundary(x, y, node, tolerance) {
            return true;
        }
    }
    false
}

/// Check if a point is within tolerance of a node's boundary
fn point_touches_node_boundary(x: f64, y: f64, node: &NodeBounds, tolerance: f64) -> bool {
    let left = node.x;
    let right = node.x + node.width;
    let top = node.y;
    let bottom = node.y + node.height;

    // Check if point is near any of the four sides
    let near_left =
        (x - left).abs() <= tolerance && y >= top - tolerance && y <= bottom + tolerance;
    let near_right =
        (x - right).abs() <= tolerance && y >= top - tolerance && y <= bottom + tolerance;
    let near_top = (y - top).abs() <= tolerance && x >= left - tolerance && x <= right + tolerance;
    let near_bottom =
        (y - bottom).abs() <= tolerance && x >= left - tolerance && x <= right + tolerance;

    near_left || near_right || near_top || near_bottom
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::Level;
    use crate::render::svg::structure::{ShapeCounts, ZOrderAnalysis};

    fn make_structure(nodes: usize, edges: usize, labels: Vec<&str>) -> SvgStructure {
        use crate::render::svg::structure::{EdgeGeometry, FontAnalysis, StrokeAnalysis};
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
            stroke_analysis: StrokeAnalysis::default(),
            edge_geometry: EdgeGeometry::default(),
            font_analysis: FontAnalysis::default(),
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
