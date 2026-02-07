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
    check_layout_pattern(selkie, reference, &mut issues);
    check_node_containment(selkie, reference, &mut issues);
    check_shape_counts(selkie, reference, &mut issues);
    check_z_order(selkie, reference, &mut issues);
    check_stroke_widths(selkie, reference, &mut issues);
    check_edge_attachments(selkie, reference, &mut issues);
    check_edge_node_connectivity(selkie, reference, &mut issues);
    check_font_styles(selkie, reference, &mut issues);

    // Selkie-only checks (don't compare to reference, check for rendering issues)
    check_text_overflow(selkie, &mut issues);
    check_element_spacing(selkie, &mut issues);

    // Comparative text placement checks
    check_text_placement(selkie, reference, &mut issues);

    // INFO checks - acceptable variations
    check_extra_labels(selkie, reference, &mut issues);
    check_markers(selkie, reference, &mut issues);
    check_colors(selkie, reference, &mut issues);

    // ERROR checks - text visibility issues (CSS fill override)
    check_text_visibility(selkie, &mut issues);

    // WARNING checks - text fill color mismatches
    check_text_fill_colors(selkie, reference, &mut issues);

    // WARNING checks - layout/aspect ratio differences
    check_aspect_ratio(selkie, reference, &mut issues);
    check_vertical_distribution(selkie, reference, &mut issues);
    check_composite_state_structure(selkie, reference, &mut issues);
    check_composite_centering(selkie, reference, &mut issues);
    check_nested_composite_centering(selkie, reference, &mut issues);

    issues
}

/// Check for text visibility issues where CSS fill rules override inline fill attributes
/// This causes text to have unexpected colors that may be invisible against backgrounds
fn check_text_visibility(selkie: &SvgStructure, issues: &mut Vec<Issue>) {
    let visibility_issues = &selkie.color_analysis.text_visibility_issues;

    if !visibility_issues.is_empty() {
        let mut messages = Vec::new();

        for issue in visibility_issues {
            let bg_info = if let Some(ref bg) = issue.background_fill {
                format!(" (background: {})", bg)
            } else {
                String::new()
            };

            messages.push(format!(
                "Text '{}' has class '{}' with CSS fill '{}' overriding inline fill '{}'{}",
                issue.text,
                issue.css_class,
                issue.css_fill,
                issue.inline_fill.as_deref().unwrap_or("none"),
                bg_info
            ));
        }

        issues.push(Issue::error(
            "text_visibility",
            format!(
                "TEXT VISIBILITY ISSUE: CSS fill rules override inline text colors, potentially making text invisible:\n  {}",
                messages.join("\n  ")
            ),
        ));
    }
}

/// Check for text fill color mismatches between selkie and reference
/// Reference may use CSS/foreignObject for text colors, selkie uses inline fill
fn check_text_fill_colors(
    selkie: &SvgStructure,
    reference: &SvgStructure,
    issues: &mut Vec<Issue>,
) {
    // Extract text fill colors from raw SVG
    let selkie_text_fills = extract_text_fill_colors(&selkie.raw_svg);
    let ref_text_fills = extract_text_fill_colors(&reference.raw_svg);

    // Check if selkie has fill colors on text that reference doesn't
    let selkie_set: std::collections::HashSet<_> = selkie_text_fills.iter().collect();
    let ref_set: std::collections::HashSet<_> = ref_text_fills.iter().collect();

    let extra_fills: Vec<_> = selkie_set.difference(&ref_set).cloned().collect();

    if !extra_fills.is_empty() {
        // Check if we're using white text where reference might use dark text
        let has_white_text = extra_fills.iter().any(|c| {
            let c = c.to_lowercase();
            c == "#fff" || c == "#ffffff" || c == "white"
        });

        if has_white_text {
            issues.push(Issue::warning(
                "text_fill_mismatch",
                format!(
                    "Text fill color mismatch: selkie uses {:?} but reference text has no inline fill (uses CSS/foreignObject). Reference text color is typically #333 (dark) via CSS .label class.",
                    extra_fills
                ),
            ));
        }
    }
}

/// Extract fill colors from text elements in raw SVG
fn extract_text_fill_colors(svg: &str) -> Vec<String> {
    let mut fills = Vec::new();

    // Simple extraction for text element fills
    // Look for <text ... fill="..." ...>
    for part in svg.split("<text") {
        if let Some(tag_end) = part.find('>') {
            let tag_content = &part[..tag_end];
            // Extract fill attribute
            if let Some(fill_start) = tag_content.find("fill=\"") {
                let after_fill = &tag_content[fill_start + 6..];
                if let Some(fill_end) = after_fill.find('"') {
                    let fill_value = &after_fill[..fill_end];
                    if !fill_value.is_empty() && fill_value != "none" {
                        fills.push(fill_value.to_lowercase());
                    }
                }
            }
        }
    }

    fills.sort();
    fills.dedup();
    fills
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

/// Count the number of distinct rows in a layout by clustering Y positions
fn count_layout_rows(node_bounds: &[NodeBounds]) -> usize {
    if node_bounds.is_empty() {
        return 0;
    }

    // Cluster Y positions with a tolerance of 20px
    let tolerance = 20.0;
    let mut y_positions: Vec<f64> = node_bounds.iter().map(|b| b.y).collect();
    y_positions.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let mut rows = 1;
    let mut last_y = y_positions[0];

    for &y in &y_positions[1..] {
        if (y - last_y).abs() > tolerance {
            rows += 1;
            last_y = y;
        }
    }

    rows
}

/// Check layout pattern - ERROR if significantly different row/column arrangement
fn check_layout_pattern(selkie: &SvgStructure, reference: &SvgStructure, issues: &mut Vec<Issue>) {
    let selkie_bounds = &selkie.edge_geometry.node_bounds;
    let ref_bounds = &reference.edge_geometry.node_bounds;

    // Skip if no nodes to compare
    if selkie_bounds.is_empty() || ref_bounds.is_empty() {
        return;
    }

    let selkie_rows = count_layout_rows(selkie_bounds);
    let ref_rows = count_layout_rows(ref_bounds);

    // If reference has 1 row but selkie has multiple, that's a layout error
    if ref_rows == 1 && selkie_rows > 1 {
        issues.push(
            Issue::error(
                "layout_pattern",
                format!(
                    "Layout differs: reference has {} row (horizontal layout), selkie has {} rows (vertical stacking)",
                    ref_rows, selkie_rows
                ),
            )
            .with_values(format!("{} rows", ref_rows), format!("{} rows", selkie_rows)),
        );
    } else if ref_rows > selkie_rows && selkie_rows == 1 {
        issues.push(
            Issue::error(
                "layout_pattern",
                format!(
                    "Layout differs: reference has {} rows (vertical layout), selkie has {} row (horizontal)",
                    ref_rows, selkie_rows
                ),
            )
            .with_values(format!("{} rows", ref_rows), format!("{} rows", selkie_rows)),
        );
    } else if (selkie_rows as i32 - ref_rows as i32).abs() > 1 {
        issues.push(
            Issue::warning(
                "layout_pattern",
                format!(
                    "Row count differs: reference has {} rows, selkie has {} rows",
                    ref_rows, selkie_rows
                ),
            )
            .with_values(
                format!("{} rows", ref_rows),
                format!("{} rows", selkie_rows),
            ),
        );
    }
}

/// Check if a node is a container (composite block, cluster, etc.)
fn is_container_node(bounds: &NodeBounds) -> bool {
    let id = bounds.id.to_lowercase();
    // Selkie patterns
    id.contains("composite")
        || id.contains("cluster")
        || id.contains("container")
        // Mermaid patterns: auto-generated IDs like "id-w3hwz1x9xf8-1"
        || (bounds.id.starts_with("id-")
            && bounds.id.len() > 10
            && bounds.id.chars().skip(3).take(8).all(|c| c.is_alphanumeric()))
}

/// Check if node a contains node b geometrically
fn node_contains(container: &NodeBounds, inner: &NodeBounds) -> bool {
    // Check if inner is fully within container bounds (with small tolerance)
    let tolerance = 5.0;
    inner.x >= container.x - tolerance
        && inner.y >= container.y - tolerance
        && inner.x + inner.width <= container.x + container.width + tolerance
        && inner.y + inner.height <= container.y + container.height + tolerance
}

/// Count how many nodes are contained within containers
fn count_contained_nodes(node_bounds: &[NodeBounds]) -> usize {
    let mut count = 0;
    for container in node_bounds {
        if is_container_node(container) {
            for inner in node_bounds {
                if inner.id != container.id && node_contains(container, inner) {
                    count += 1;
                }
            }
        }
    }
    count
}

/// Check node containment - ERROR if nodes should be nested but aren't
fn check_node_containment(
    selkie: &SvgStructure,
    reference: &SvgStructure,
    issues: &mut Vec<Issue>,
) {
    let selkie_bounds = &selkie.edge_geometry.node_bounds;
    let ref_bounds = &reference.edge_geometry.node_bounds;

    // Skip if no nodes to compare
    if selkie_bounds.is_empty() || ref_bounds.is_empty() {
        return;
    }

    let selkie_contained = count_contained_nodes(selkie_bounds);
    let ref_contained = count_contained_nodes(ref_bounds);

    // If reference has nested nodes but selkie doesn't, that's an error
    if ref_contained > 0 && selkie_contained == 0 {
        issues.push(
            Issue::error(
                "node_containment",
                format!(
                    "Containment missing: reference has {} nodes nested inside containers, selkie has {}",
                    ref_contained, selkie_contained
                ),
            )
            .with_values(
                format!("{} nested nodes", ref_contained),
                format!("{} nested nodes", selkie_contained),
            ),
        );
    } else if ref_contained > selkie_contained && selkie_contained > 0 {
        issues.push(
            Issue::warning(
                "node_containment",
                format!(
                    "Containment differs: reference has {} nested nodes, selkie has {}",
                    ref_contained, selkie_contained
                ),
            )
            .with_values(
                format!("{} nested nodes", ref_contained),
                format!("{} nested nodes", selkie_contained),
            ),
        );
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

/// Check colors - WARNING if fill colors significantly different
fn check_colors(selkie: &SvgStructure, reference: &SvgStructure, issues: &mut Vec<Issue>) {
    let selkie_fills: HashSet<_> = selkie.color_analysis.fill_colors.iter().collect();
    let ref_fills: HashSet<_> = reference.color_analysis.fill_colors.iter().collect();

    // Find colors in reference that are missing in selkie
    let missing_fills: Vec<_> = ref_fills.difference(&selkie_fills).cloned().collect();

    // Find colors in selkie that aren't in reference
    let extra_fills: Vec<_> = selkie_fills.difference(&ref_fills).cloned().collect();

    // Report as warning if there are significant color differences
    if !missing_fills.is_empty() || !extra_fills.is_empty() {
        let mut msg = String::new();

        if !missing_fills.is_empty() {
            msg.push_str(&format!("Missing fill colors: {:?}", missing_fills));
        }
        if !extra_fills.is_empty() {
            if !msg.is_empty() {
                msg.push_str("; ");
            }
            msg.push_str(&format!("Extra fill colors: {:?}", extra_fills));
        }

        // Calculate color match percentage
        let total_unique = selkie_fills.len().max(ref_fills.len());
        let matching = selkie_fills.intersection(&ref_fills).count();
        let match_pct = if total_unique > 0 {
            (matching as f64 / total_unique as f64) * 100.0
        } else {
            100.0
        };

        if match_pct < 50.0 {
            // Significant color mismatch
            issues.push(
                Issue::warning(
                    "colors",
                    format!("Color mismatch ({:.0}% match): {}", match_pct, msg),
                )
                .with_values(
                    format!("{:?}", reference.color_analysis.fill_colors),
                    format!("{:?}", selkie.color_analysis.fill_colors),
                ),
            );
        } else if match_pct < 80.0 {
            // Moderate color difference
            issues.push(Issue::info(
                "colors",
                format!("Color differences ({:.0}% match): {}", match_pct, msg),
            ));
        }
        // If >= 80% match, don't report (minor variations are acceptable)
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

/// Check if a point is within tolerance of a node's boundary or inside it.
/// Architecture diagrams route edges through invisible junction nodes (to their
/// center), so we also accept points that are inside the node bounds.
fn point_touches_node_boundary(x: f64, y: f64, node: &NodeBounds, tolerance: f64) -> bool {
    let left = node.x;
    let right = node.x + node.width;
    let top = node.y;
    let bottom = node.y + node.height;

    // Check if point is inside the node bounds (for junctions and similar)
    let inside = x >= left - tolerance
        && x <= right + tolerance
        && y >= top - tolerance
        && y <= bottom + tolerance;

    if inside {
        return true;
    }

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

/// Check for text that overflows/escapes its containing node
/// This detects a common rendering issue where text is too long for its box
fn check_text_overflow(selkie: &SvgStructure, issues: &mut Vec<Issue>) {
    let node_bounds = &selkie.edge_geometry.node_bounds;

    if node_bounds.is_empty() {
        return;
    }

    let mut overflow_issues = Vec::new();
    let tolerance = 5.0; // Allow small tolerance

    // Parse SVG directly to check text bounds accurately
    if let Ok(doc) = roxmltree::Document::parse(&selkie.raw_svg) {
        for text_node in doc.descendants().filter(|n| n.tag_name().name() == "text") {
            // Get text y coordinate
            let text_y = text_node
                .attribute("y")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);

            // Count tspans and check for dy attributes
            let tspans: Vec<_> = text_node
                .descendants()
                .filter(|n| n.tag_name().name() == "tspan")
                .collect();

            if tspans.is_empty() {
                continue; // Skip single-line text without tspans
            }

            // Calculate total dy offset for multi-line text
            let font_size = 16.0; // Default font size
            let mut total_dy = 0.0;
            for tspan in &tspans {
                if let Some(dy) = tspan.attribute("dy") {
                    // Parse dy value (e.g., "1em", "1.1em")
                    let dy_value = if dy.ends_with("em") {
                        dy.trim_end_matches("em").parse::<f64>().unwrap_or(0.0) * font_size
                    } else {
                        dy.parse::<f64>().unwrap_or(0.0)
                    };
                    total_dy += dy_value;
                }
            }

            // Calculate actual text bottom (y + all dy offsets + descender space)
            let text_bottom = text_y + total_dy + font_size * 0.2;

            // Get parent group transform to find absolute position
            let mut group_y = 0.0;
            let mut current = text_node.parent();
            while let Some(parent) = current {
                if let Some(transform) = parent.attribute("transform") {
                    if let Some(ty) = parse_translate_y(transform) {
                        group_y += ty;
                    }
                }
                current = parent.parent();
            }

            let absolute_text_bottom = group_y + text_bottom;

            // Find containing node by checking if it contains the text's position
            let text_x = text_node
                .attribute("x")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            let mut group_x = 0.0;
            let mut current2 = text_node.parent();
            while let Some(parent) = current2 {
                if let Some(transform) = parent.attribute("transform") {
                    if let Some(tx) = parse_translate_x(transform) {
                        group_x += tx;
                    }
                }
                current2 = parent.parent();
            }
            let absolute_text_x = group_x + text_x;

            // Find the containing node
            let containing_node = node_bounds.iter().find(|n| {
                absolute_text_x >= n.x - tolerance
                    && absolute_text_x <= n.x + n.width + tolerance
                    && group_y + text_y >= n.y - tolerance
                    && group_y + text_y <= n.y + n.height + tolerance
            });

            if let Some(node) = containing_node {
                let node_bottom = node.y + node.height;
                let overflow_bottom = absolute_text_bottom - node_bottom;

                if overflow_bottom > tolerance {
                    // Get text content for preview
                    let text_content: String = text_node
                        .descendants()
                        .filter_map(|n| n.text())
                        .collect::<Vec<_>>()
                        .join(" ");
                    let text_preview = if text_content.len() > 30 {
                        format!("{}...", &text_content[..30])
                    } else {
                        text_content
                    };

                    overflow_issues.push(format!(
                        "Text \"{}\" overflows bottom by {:.0}px (node height: {:.0}, text needs: {:.0})",
                        text_preview,
                        overflow_bottom,
                        node.height,
                        absolute_text_bottom - node.y
                    ));
                }
            }
        }
    }

    if !overflow_issues.is_empty() {
        issues.push(Issue::warning(
            "text_overflow",
            format!(
                "TEXT OVERFLOW DETECTED ({} issues):\n  {}",
                overflow_issues.len(),
                overflow_issues.join("\n  ")
            ),
        ));
    }
}

/// Parse x-coordinate from translate transform
fn parse_translate_x(transform: &str) -> Option<f64> {
    if let Some(start) = transform.find("translate(") {
        let rest = &transform[start + 10..];
        if let Some(end) = rest.find(')') {
            let coords = &rest[..end];
            let parts: Vec<&str> = coords.split([',', ' ']).filter(|s| !s.is_empty()).collect();
            if !parts.is_empty() {
                return parts[0].trim().parse::<f64>().ok();
            }
        }
    }
    None
}

/// Parse y-coordinate from translate transform
fn parse_translate_y(transform: &str) -> Option<f64> {
    if let Some(start) = transform.find("translate(") {
        let rest = &transform[start + 10..];
        if let Some(end) = rest.find(')') {
            let coords = &rest[..end];
            let parts: Vec<&str> = coords.split([',', ' ']).filter(|s| !s.is_empty()).collect();
            if parts.len() >= 2 {
                return parts[1].trim().parse::<f64>().ok();
            }
        }
    }
    None
}

/// Check for element spacing issues (overlapping or too close)
fn check_element_spacing(selkie: &SvgStructure, issues: &mut Vec<Issue>) {
    let node_bounds = &selkie.edge_geometry.node_bounds;

    if node_bounds.len() < 2 {
        return;
    }

    let mut spacing_issues = Vec::new();
    let min_spacing = 5.0; // Minimum expected spacing between elements

    // Check for overlapping nodes
    for i in 0..node_bounds.len() {
        for j in (i + 1)..node_bounds.len() {
            let a = &node_bounds[i];
            let b = &node_bounds[j];

            // Check for overlap
            let overlap_x = (a.x < b.x + b.width) && (a.x + a.width > b.x);
            let overlap_y = (a.y < b.y + b.height) && (a.y + a.height > b.y);

            if overlap_x && overlap_y {
                // Calculate overlap amount
                let overlap_width = (a.x + a.width).min(b.x + b.width) - a.x.max(b.x);
                let overlap_height = (a.y + a.height).min(b.y + b.height) - a.y.max(b.y);

                // Only report significant overlaps (not just touching)
                if overlap_width > min_spacing && overlap_height > min_spacing {
                    spacing_issues.push(format!(
                        "Nodes \"{}\" and \"{}\" overlap by {:.0}x{:.0}px",
                        if a.id.is_empty() {
                            format!("({:.0},{:.0})", a.x, a.y)
                        } else {
                            a.id.clone()
                        },
                        if b.id.is_empty() {
                            format!("({:.0},{:.0})", b.x, b.y)
                        } else {
                            b.id.clone()
                        },
                        overlap_width,
                        overlap_height
                    ));
                }
            }
        }
    }

    // Check for very close but not overlapping elements (potential spacing inconsistency)
    let mut close_pairs = 0;
    for i in 0..node_bounds.len() {
        for j in (i + 1)..node_bounds.len() {
            let a = &node_bounds[i];
            let b = &node_bounds[j];

            // Calculate minimum distance between nodes
            let dx = if a.x + a.width < b.x {
                b.x - (a.x + a.width)
            } else if b.x + b.width < a.x {
                a.x - (b.x + b.width)
            } else {
                0.0 // Overlapping in x
            };

            let dy = if a.y + a.height < b.y {
                b.y - (a.y + a.height)
            } else if b.y + b.height < a.y {
                a.y - (b.y + b.height)
            } else {
                0.0 // Overlapping in y
            };

            let distance = (dx * dx + dy * dy).sqrt();

            // Elements very close but not overlapping (possible spacing issue)
            if distance > 0.0 && distance < min_spacing {
                close_pairs += 1;
            }
        }
    }

    if close_pairs > 3 {
        spacing_issues.push(format!(
            "{} pairs of elements are very close together (< {}px apart)",
            close_pairs, min_spacing
        ));
    }

    if !spacing_issues.is_empty() {
        issues.push(Issue::warning(
            "element_spacing",
            format!(
                "SPACING ISSUES DETECTED ({} issues):\n  {}",
                spacing_issues.len(),
                spacing_issues.join("\n  ")
            ),
        ));
    }
}

/// Check text placement within nodes by comparing vertical centering
/// This detects issues where text is not properly centered/positioned within containing nodes
fn check_text_placement(selkie: &SvgStructure, reference: &SvgStructure, issues: &mut Vec<Issue>) {
    let selkie_text = &selkie.edge_geometry.text_bounds;
    let selkie_nodes = &selkie.edge_geometry.node_bounds;
    let ref_text = &reference.edge_geometry.text_bounds;
    let ref_nodes = &reference.edge_geometry.node_bounds;

    // Skip if not enough data to compare
    if selkie_text.is_empty() || ref_text.is_empty() {
        return;
    }

    // Calculate average vertical offset (text_y relative to containing node top) for both
    let selkie_offsets = calculate_text_vertical_offsets(selkie_text, selkie_nodes);
    let ref_offsets = calculate_text_vertical_offsets(ref_text, ref_nodes);

    if selkie_offsets.is_empty() || ref_offsets.is_empty() {
        return;
    }

    // Calculate average relative vertical position (0 = top, 0.5 = center, 1 = bottom)
    let selkie_avg_rel: f64 =
        selkie_offsets.iter().map(|(_, rel, _)| rel).sum::<f64>() / selkie_offsets.len() as f64;
    let ref_avg_rel: f64 =
        ref_offsets.iter().map(|(_, rel, _)| rel).sum::<f64>() / ref_offsets.len() as f64;

    // Difference in relative position - significant if > 0.15 (15% of node height)
    let rel_diff = (selkie_avg_rel - ref_avg_rel).abs();

    if rel_diff > 0.15 {
        let selkie_pos = if selkie_avg_rel < 0.35 {
            "near top"
        } else if selkie_avg_rel > 0.65 {
            "near bottom"
        } else {
            "centered"
        };
        let ref_pos = if ref_avg_rel < 0.35 {
            "near top"
        } else if ref_avg_rel > 0.65 {
            "near bottom"
        } else {
            "centered"
        };

        issues.push(
            Issue::warning(
                "text_placement",
                format!(
                    "Text vertical placement differs: selkie positions text {} ({:.0}% from top), \
                     reference positions text {} ({:.0}% from top). Difference: {:.0}%",
                    selkie_pos,
                    selkie_avg_rel * 100.0,
                    ref_pos,
                    ref_avg_rel * 100.0,
                    rel_diff * 100.0
                ),
            )
            .with_values(
                format!("{:.0}% from top", ref_avg_rel * 100.0),
                format!("{:.0}% from top", selkie_avg_rel * 100.0),
            ),
        );
    }

    // Also check for specific text elements with significant placement differences
    let mut placement_mismatches = Vec::new();

    for (selkie_offset, selkie_rel, selkie_label) in &selkie_offsets {
        // Find matching text in reference by label content
        let matching_ref = ref_offsets.iter().find(|(_, _, ref_label)| {
            // Match by first word or full content
            let selkie_first = selkie_label.split_whitespace().next().unwrap_or("");
            let ref_first = ref_label.split_whitespace().next().unwrap_or("");
            selkie_first == ref_first || selkie_label == ref_label
        });

        if let Some((ref_offset, ref_rel, _)) = matching_ref {
            let offset_diff = (selkie_offset - ref_offset).abs();
            let rel_diff = (selkie_rel - ref_rel).abs();

            // Report if absolute offset differs by > 10px OR relative position by > 20%
            if offset_diff > 10.0 || rel_diff > 0.2 {
                // Truncate label for display
                let label_preview = if selkie_label.len() > 20 {
                    format!("{}...", &selkie_label[..20])
                } else {
                    selkie_label.clone()
                };

                placement_mismatches.push(format!(
                    "\"{}\" at y-offset {:.0}px ({:.0}%) vs reference {:.0}px ({:.0}%)",
                    label_preview,
                    selkie_offset,
                    selkie_rel * 100.0,
                    ref_offset,
                    ref_rel * 100.0
                ));
            }
        }
    }

    if !placement_mismatches.is_empty() {
        issues.push(Issue::warning(
            "text_placement_details",
            format!(
                "TEXT PLACEMENT MISMATCHES ({} issues):\n  {}",
                placement_mismatches.len(),
                placement_mismatches.join("\n  ")
            ),
        ));
    }
}

/// Calculate vertical offsets for text elements relative to their containing nodes
/// Returns Vec of (absolute_offset_from_top, relative_position_0_to_1, label_text)
fn calculate_text_vertical_offsets(
    text_bounds: &[crate::render::svg::structure::TextBounds],
    node_bounds: &[NodeBounds],
) -> Vec<(f64, f64, String)> {
    let mut offsets = Vec::new();

    for text in text_bounds {
        // Find containing node - first by parent ID, then by geometric containment
        let mut containing_node = None;

        // Try parent ID match first
        if let Some(ref parent_id) = text.parent_node_id {
            containing_node = node_bounds.iter().find(|n| n.id == *parent_id);
        }

        // Fall back to geometric containment if parent ID match failed or wasn't available
        if containing_node.is_none() {
            containing_node = node_bounds.iter().find(|n| {
                let x_in = text.x >= n.x - 10.0 && text.x <= n.x + n.width + 10.0;
                let y_in = text.y >= n.y - 10.0 && text.y <= n.y + n.height + 10.0;
                x_in && y_in
            });
        }

        if let Some(node) = containing_node {
            // Calculate text vertical center relative to node
            let text_center_y = text.y + text.height / 2.0;
            let node_top = node.y;
            let node_height = node.height;

            // Absolute offset from top of node
            let offset = text_center_y - node_top;

            // Relative position (0 = top, 0.5 = center, 1 = bottom)
            let relative = if node_height > 0.0 {
                offset / node_height
            } else {
                0.5
            };

            offsets.push((offset, relative, text.text.clone()));
        }
    }

    offsets
}

/// Check aspect ratio differences - ERROR if orientation flipped (portrait vs landscape)
fn check_aspect_ratio(selkie: &SvgStructure, reference: &SvgStructure, issues: &mut Vec<Issue>) {
    if reference.width <= 0.0 || reference.height <= 0.0 {
        return;
    }
    if selkie.width <= 0.0 || selkie.height <= 0.0 {
        return;
    }

    let ref_aspect = reference.width / reference.height;
    let selkie_aspect = selkie.width / selkie.height;

    // Categorize orientation
    let ref_orientation = if ref_aspect > 1.2 {
        "landscape"
    } else if ref_aspect < 0.8 {
        "portrait"
    } else {
        "square"
    };

    let selkie_orientation = if selkie_aspect > 1.2 {
        "landscape"
    } else if selkie_aspect < 0.8 {
        "portrait"
    } else {
        "square"
    };

    // Report if orientation category differs
    if ref_orientation != selkie_orientation {
        issues.push(
            Issue::error(
                "aspect_ratio",
                format!(
                    "Diagram orientation differs: reference is {} ({}x{}, ratio {:.2}), selkie is {} ({}x{}, ratio {:.2})",
                    ref_orientation,
                    reference.width as i32,
                    reference.height as i32,
                    ref_aspect,
                    selkie_orientation,
                    selkie.width as i32,
                    selkie.height as i32,
                    selkie_aspect
                ),
            )
            .with_values(
                format!("{} ({:.2})", ref_orientation, ref_aspect),
                format!("{} ({:.2})", selkie_orientation, selkie_aspect),
            ),
        );
    } else {
        // Same orientation but check for significant aspect ratio difference
        let ratio_diff = (ref_aspect - selkie_aspect).abs() / ref_aspect;
        if ratio_diff > 0.3 {
            issues.push(
                Issue::warning(
                    "aspect_ratio",
                    format!(
                        "Aspect ratio differs significantly: reference {:.2}, selkie {:.2} ({:.0}% difference)",
                        ref_aspect, selkie_aspect, ratio_diff * 100.0
                    ),
                )
                .with_values(format!("{:.2}", ref_aspect), format!("{:.2}", selkie_aspect)),
            );
        }
    }
}

/// Check vertical distribution of nodes - WARNING if selkie stacks more vertically
fn check_vertical_distribution(
    selkie: &SvgStructure,
    reference: &SvgStructure,
    issues: &mut Vec<Issue>,
) {
    let selkie_nodes = &selkie.edge_geometry.node_bounds;
    let ref_nodes = &reference.edge_geometry.node_bounds;

    if selkie_nodes.len() < 3 || ref_nodes.len() < 3 {
        return;
    }

    // Calculate Y-spread (range of Y positions)
    let selkie_y_vals: Vec<f64> = selkie_nodes.iter().map(|n| n.y).collect();
    let ref_y_vals: Vec<f64> = ref_nodes.iter().map(|n| n.y).collect();

    let selkie_y_min = selkie_y_vals.iter().cloned().fold(f64::MAX, f64::min);
    let selkie_y_max = selkie_y_vals.iter().cloned().fold(f64::MIN, f64::max);
    let ref_y_min = ref_y_vals.iter().cloned().fold(f64::MAX, f64::min);
    let ref_y_max = ref_y_vals.iter().cloned().fold(f64::MIN, f64::max);

    let selkie_y_spread = selkie_y_max - selkie_y_min;
    let ref_y_spread = ref_y_max - ref_y_min;

    // Calculate X-spread
    let selkie_x_vals: Vec<f64> = selkie_nodes.iter().map(|n| n.x).collect();
    let ref_x_vals: Vec<f64> = ref_nodes.iter().map(|n| n.x).collect();

    let selkie_x_min = selkie_x_vals.iter().cloned().fold(f64::MAX, f64::min);
    let selkie_x_max = selkie_x_vals.iter().cloned().fold(f64::MIN, f64::max);
    let ref_x_min = ref_x_vals.iter().cloned().fold(f64::MAX, f64::min);
    let ref_x_max = ref_x_vals.iter().cloned().fold(f64::MIN, f64::max);

    let selkie_x_spread = selkie_x_max - selkie_x_min;
    let ref_x_spread = ref_x_max - ref_x_min;

    // Compare Y/X spread ratios (high ratio = more vertical stacking)
    let selkie_ratio = if selkie_x_spread > 0.0 {
        selkie_y_spread / selkie_x_spread
    } else {
        selkie_y_spread
    };
    let ref_ratio = if ref_x_spread > 0.0 {
        ref_y_spread / ref_x_spread
    } else {
        ref_y_spread
    };

    // If selkie has much higher Y/X ratio, it's stacking more vertically
    if selkie_ratio > ref_ratio * 1.5 && selkie_y_spread > ref_y_spread * 1.2 {
        issues.push(
            Issue::warning(
                "vertical_distribution",
                format!(
                    "Nodes are stacked more vertically: selkie Y-spread {:.0}px (ratio {:.2}), reference Y-spread {:.0}px (ratio {:.2}). Selkie is {:.0}% taller in node distribution.",
                    selkie_y_spread,
                    selkie_ratio,
                    ref_y_spread,
                    ref_ratio,
                    ((selkie_y_spread - ref_y_spread) / ref_y_spread) * 100.0
                ),
            )
            .with_values(
                format!("Y-spread: {:.0}px", ref_y_spread),
                format!("Y-spread: {:.0}px", selkie_y_spread),
            ),
        );
    }

    // Count nodes per "row" (cluster by Y position with tolerance)
    let selkie_rows = count_y_clusters(&selkie_y_vals, 30.0);
    let ref_rows = count_y_clusters(&ref_y_vals, 30.0);

    if selkie_rows != ref_rows {
        let selkie_per_row = selkie_nodes.len() as f64 / selkie_rows as f64;
        let ref_per_row = ref_nodes.len() as f64 / ref_rows as f64;

        issues.push(
            Issue::info(
                "row_distribution",
                format!(
                    "Node row distribution differs: reference has {} rows (~{:.1} nodes/row), selkie has {} rows (~{:.1} nodes/row)",
                    ref_rows, ref_per_row, selkie_rows, selkie_per_row
                ),
            ),
        );
    }
}

/// Count clusters of Y values (nodes on same "row")
fn count_y_clusters(y_vals: &[f64], tolerance: f64) -> usize {
    if y_vals.is_empty() {
        return 0;
    }

    let mut sorted = y_vals.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let mut clusters = 1;
    let mut last_y = sorted[0];

    for &y in &sorted[1..] {
        if (y - last_y).abs() > tolerance {
            clusters += 1;
            last_y = y;
        }
    }

    clusters
}

/// Check composite state structure - compare nesting patterns for state diagrams
fn check_composite_state_structure(
    selkie: &SvgStructure,
    reference: &SvgStructure,
    issues: &mut Vec<Issue>,
) {
    // Extract composite state info from raw SVG
    let selkie_composites = extract_composite_states(&selkie.raw_svg);
    let ref_composites = extract_composite_states(&reference.raw_svg);

    if selkie_composites.is_empty() && ref_composites.is_empty() {
        return;
    }

    // Compare composite state counts
    if selkie_composites.len() != ref_composites.len() {
        issues.push(
            Issue::warning(
                "composite_structure",
                format!(
                    "Composite state count differs: reference has {}, selkie has {}",
                    ref_composites.len(),
                    selkie_composites.len()
                ),
            )
            .with_values(
                ref_composites.len().to_string(),
                selkie_composites.len().to_string(),
            ),
        );
    }

    // Check if composite states are named the same
    let selkie_names: HashSet<_> = selkie_composites.iter().map(|c| c.id.as_str()).collect();
    let ref_names: HashSet<_> = ref_composites.iter().map(|c| c.id.as_str()).collect();

    let missing: Vec<_> = ref_names.difference(&selkie_names).collect();
    let extra: Vec<_> = selkie_names.difference(&ref_names).collect();

    if !missing.is_empty() {
        issues.push(Issue::warning(
            "composite_structure",
            format!("Missing composite states: {:?}", missing),
        ));
    }

    if !extra.is_empty() {
        issues.push(Issue::info(
            "composite_structure",
            format!("Extra composite states in selkie: {:?}", extra),
        ));
    }

    // Compare composite state sizes (width/height ratios)
    for ref_comp in &ref_composites {
        if let Some(selkie_comp) = selkie_composites.iter().find(|c| c.id == ref_comp.id) {
            // Compare dimensions
            let width_diff = (selkie_comp.width - ref_comp.width).abs();
            let height_diff = (selkie_comp.height - ref_comp.height).abs();

            let width_pct = if ref_comp.width > 0.0 {
                width_diff / ref_comp.width * 100.0
            } else {
                0.0
            };
            let height_pct = if ref_comp.height > 0.0 {
                height_diff / ref_comp.height * 100.0
            } else {
                0.0
            };

            if width_pct > 30.0 || height_pct > 30.0 {
                issues.push(
                    Issue::warning(
                        "composite_size",
                        format!(
                            "Composite '{}' size differs: reference {}x{}, selkie {}x{} (width {:.0}% diff, height {:.0}% diff)",
                            ref_comp.id,
                            ref_comp.width as i32,
                            ref_comp.height as i32,
                            selkie_comp.width as i32,
                            selkie_comp.height as i32,
                            width_pct,
                            height_pct
                        ),
                    )
                    .with_values(
                        format!("{}x{}", ref_comp.width as i32, ref_comp.height as i32),
                        format!("{}x{}", selkie_comp.width as i32, selkie_comp.height as i32),
                    ),
                );
            }
        }
    }
}

/// Composite state info extracted from SVG
#[derive(Debug, Clone)]
struct CompositeState {
    id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

/// Extract composite state information from raw SVG
fn extract_composite_states(svg: &str) -> Vec<CompositeState> {
    let mut composites = Vec::new();

    // Look for composite state patterns in both mermaid and selkie SVGs
    // Mermaid pattern: <g class="... statediagram-cluster ..." id="StateName">
    // Selkie pattern: <g id="state-CompositeName" class="...">

    if let Ok(doc) = roxmltree::Document::parse(svg) {
        for node in doc.descendants() {
            if node.tag_name().name() != "g" {
                continue;
            }

            // Check for mermaid composite (statediagram-cluster class)
            let class = node.attribute("class").unwrap_or("");
            let is_mermaid_composite = class.contains("statediagram-cluster");

            // Check for selkie composite (id contains "Composite" or has composite children)
            let id = node.attribute("id").unwrap_or("");
            let is_selkie_composite = !id.is_empty()
                && node.descendants().any(|n| {
                    n.attribute("class")
                        .map(|c| c.contains("composite"))
                        .unwrap_or(false)
                });

            if is_mermaid_composite || is_selkie_composite {
                // Extract composite ID (normalize by removing prefixes)
                let composite_id = if is_mermaid_composite {
                    node.attribute("id")
                        .or_else(|| node.attribute("data-id"))
                        .unwrap_or("")
                        .to_string()
                } else {
                    // Selkie uses "composite-StateName" format
                    id.trim_start_matches("composite-")
                        .trim_start_matches("state-")
                        .to_string()
                };

                if composite_id.is_empty()
                    || composite_id.contains("start")
                    || composite_id.contains("end")
                {
                    continue;
                }

                // Find the outer rect for dimensions and position
                let mut x = 0.0;
                let mut y = 0.0;
                let mut width = 0.0;
                let mut height = 0.0;

                for child in node.descendants() {
                    if child.tag_name().name() == "rect" {
                        let child_class = child.attribute("class").unwrap_or("");
                        if child_class.contains("outer") || child_class.contains("composite-outer")
                        {
                            x = child
                                .attribute("x")
                                .and_then(|v| v.parse().ok())
                                .unwrap_or(0.0);
                            y = child
                                .attribute("y")
                                .and_then(|v| v.parse().ok())
                                .unwrap_or(0.0);
                            width = child
                                .attribute("width")
                                .and_then(|w| w.parse().ok())
                                .unwrap_or(0.0);
                            height = child
                                .attribute("height")
                                .and_then(|h| h.parse().ok())
                                .unwrap_or(0.0);
                            break;
                        }
                    }
                }

                // If no outer rect found, try to get dimensions from largest rect
                if width == 0.0 || height == 0.0 {
                    for child in node.descendants() {
                        if child.tag_name().name() == "rect" {
                            let w: f64 = child
                                .attribute("width")
                                .and_then(|w| w.parse().ok())
                                .unwrap_or(0.0);
                            let h: f64 = child
                                .attribute("height")
                                .and_then(|h| h.parse().ok())
                                .unwrap_or(0.0);
                            if w > width {
                                x = child
                                    .attribute("x")
                                    .and_then(|v| v.parse().ok())
                                    .unwrap_or(0.0);
                                y = child
                                    .attribute("y")
                                    .and_then(|v| v.parse().ok())
                                    .unwrap_or(0.0);
                                width = w;
                            }
                            if h > height {
                                height = h;
                            }
                        }
                    }
                }

                if width > 0.0 && height > 0.0 {
                    composites.push(CompositeState {
                        id: composite_id,
                        x,
                        y,
                        width,
                        height,
                    });
                }
            }
        }
    }

    composites
}

/// Check if composite states have their children properly centered
/// This compares the horizontal centering of child nodes within parent composites
fn check_composite_centering(
    selkie: &SvgStructure,
    reference: &SvgStructure,
    issues: &mut Vec<Issue>,
) {
    let selkie_centering = analyze_composite_centering(&selkie.raw_svg);
    let ref_centering = analyze_composite_centering(&reference.raw_svg);

    if selkie_centering.is_empty() && ref_centering.is_empty() {
        return;
    }

    // Check for centering issues in selkie
    for (composite_id, selkie_offset) in &selkie_centering {
        // Threshold for centering (pixels) - children should be within 10px of center
        let centering_threshold = 15.0;

        if selkie_offset.abs() > centering_threshold {
            // Check if reference has the same composite and is better centered
            if let Some(ref_offset) = ref_centering.get(composite_id) {
                if ref_offset.abs() < selkie_offset.abs() {
                    issues.push(
                        Issue::warning(
                            "composite_centering",
                            format!(
                                "Composite '{}' children not centered: offset {:.0}px from center (reference: {:.0}px)",
                                composite_id, selkie_offset, ref_offset
                            ),
                        )
                        .with_values(
                            format!("{:.0}px offset", ref_offset),
                            format!("{:.0}px offset", selkie_offset),
                        ),
                    );
                }
            } else {
                // Reference doesn't have this composite, just report the issue
                issues.push(Issue::warning(
                    "composite_centering",
                    format!(
                        "Composite '{}' children not centered: {:.0}px offset from center",
                        composite_id, selkie_offset
                    ),
                ));
            }
        }
    }
}

/// Analyze composite state centering - returns map of composite_id to center offset
/// Positive offset means children are to the right of center, negative means left
fn analyze_composite_centering(svg: &str) -> std::collections::HashMap<String, f64> {
    let mut centering = std::collections::HashMap::new();

    if let Ok(doc) = roxmltree::Document::parse(svg) {
        for node in doc.descendants() {
            if node.tag_name().name() != "g" {
                continue;
            }

            let class = node.attribute("class").unwrap_or("");
            let id = node.attribute("id").unwrap_or("");

            // Check for composite state (mermaid or selkie)
            let is_composite = class.contains("statediagram-cluster")
                || class.contains("composite")
                || (!id.is_empty()
                    && node.descendants().any(|n| {
                        n.attribute("class")
                            .map(|c| c.contains("composite"))
                            .unwrap_or(false)
                    }));

            if !is_composite {
                continue;
            }

            let composite_id = if class.contains("statediagram-cluster") {
                node.attribute("id")
                    .or_else(|| node.attribute("data-id"))
                    .unwrap_or("")
                    .to_string()
            } else {
                id.trim_start_matches("state-").to_string()
            };

            if composite_id.is_empty()
                || composite_id.contains("start")
                || composite_id.contains("end")
            {
                continue;
            }

            // Find the outer rect bounds
            let mut parent_x = 0.0;
            let mut parent_width = 0.0;

            for child in node.descendants() {
                if child.tag_name().name() == "rect" {
                    let child_class = child.attribute("class").unwrap_or("");
                    if child_class.contains("outer") || child_class.contains("composite-outer") {
                        parent_x = child
                            .attribute("x")
                            .and_then(|x| x.parse().ok())
                            .unwrap_or(0.0);
                        parent_width = child
                            .attribute("width")
                            .and_then(|w| w.parse().ok())
                            .unwrap_or(0.0);
                        break;
                    }
                }
            }

            // If no outer rect found, try first rect
            if parent_width == 0.0 {
                for child in node.descendants() {
                    if child.tag_name().name() == "rect" {
                        let w: f64 = child
                            .attribute("width")
                            .and_then(|w| w.parse().ok())
                            .unwrap_or(0.0);
                        if w > parent_width {
                            parent_x = child
                                .attribute("x")
                                .and_then(|x| x.parse().ok())
                                .unwrap_or(0.0);
                            parent_width = w;
                        }
                    }
                }
            }

            if parent_width == 0.0 {
                continue;
            }

            let parent_center_x = parent_x + parent_width / 2.0;

            // Find all child nodes (state nodes) within this composite
            let mut child_min_x = f64::MAX;
            let mut child_max_x = f64::MIN;
            let mut found_children = false;

            for child in node.descendants() {
                let child_class = child.attribute("class").unwrap_or("");

                // Skip the outer/inner rects of the composite itself
                if child_class.contains("composite") || child_class.contains("cluster") {
                    continue;
                }

                // Look for state nodes (rect or path elements that are state boxes)
                if (child.tag_name().name() == "rect" || child.tag_name().name() == "path")
                    && (child_class.contains("state-box") || child_class.contains("node"))
                {
                    if let Some(x) = child.attribute("x").and_then(|x| x.parse::<f64>().ok()) {
                        let width: f64 = child
                            .attribute("width")
                            .and_then(|w| w.parse().ok())
                            .unwrap_or(0.0);
                        child_min_x = child_min_x.min(x);
                        child_max_x = child_max_x.max(x + width);
                        found_children = true;
                    }
                }

                // Also check circles (start/end states)
                if child.tag_name().name() == "circle" {
                    if let Some(cx) = child.attribute("cx").and_then(|cx| cx.parse::<f64>().ok()) {
                        let r: f64 = child
                            .attribute("r")
                            .and_then(|r| r.parse().ok())
                            .unwrap_or(7.0);
                        child_min_x = child_min_x.min(cx - r);
                        child_max_x = child_max_x.max(cx + r);
                        found_children = true;
                    }
                }
            }

            if found_children && child_min_x < f64::MAX {
                let children_center_x = (child_min_x + child_max_x) / 2.0;
                let offset = children_center_x - parent_center_x;
                centering.insert(composite_id, offset);
            }
        }
    }

    centering
}

/// Check if nested composite states are centered within their parent composites
/// This uses bounding box containment to determine parent-child relationships
fn check_nested_composite_centering(
    selkie: &SvgStructure,
    reference: &SvgStructure,
    issues: &mut Vec<Issue>,
) {
    let selkie_composites = extract_composite_states(&selkie.raw_svg);
    let ref_composites = extract_composite_states(&reference.raw_svg);

    if selkie_composites.is_empty() {
        return;
    }

    // Find parent-child relationships in selkie and check centering
    let selkie_nesting = find_composite_nesting(&selkie_composites);
    let ref_nesting = find_composite_nesting(&ref_composites);

    // Check each nested relationship in selkie
    for (child_id, parent_id) in &selkie_nesting {
        let child = selkie_composites.iter().find(|c| &c.id == child_id);
        let parent = selkie_composites.iter().find(|c| &c.id == parent_id);

        if let (Some(child), Some(parent)) = (child, parent) {
            let parent_center_x = parent.x + parent.width / 2.0;
            let child_center_x = child.x + child.width / 2.0;
            let offset = child_center_x - parent_center_x;

            // Threshold for centering - child should be within 20px of parent center
            let centering_threshold = 20.0;

            if offset.abs() > centering_threshold {
                // Check if reference has better centering for this relationship
                let ref_offset = if let Some(ref_parent_id) = ref_nesting.get(child_id) {
                    if ref_parent_id == parent_id {
                        // Same parent-child relationship exists in reference
                        let ref_child = ref_composites.iter().find(|c| &c.id == child_id);
                        let ref_parent = ref_composites.iter().find(|c| &c.id == parent_id);
                        if let (Some(rc), Some(rp)) = (ref_child, ref_parent) {
                            let rp_center = rp.x + rp.width / 2.0;
                            let rc_center = rc.x + rc.width / 2.0;
                            Some(rc_center - rp_center)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                let message = if let Some(ref_off) = ref_offset {
                    format!(
                        "Nested composite '{}' not centered in '{}': {:.0}px offset (reference: {:.0}px)",
                        child_id, parent_id, offset, ref_off
                    )
                } else {
                    format!(
                        "Nested composite '{}' not centered in '{}': {:.0}px offset from center",
                        child_id, parent_id, offset
                    )
                };

                issues.push(
                    Issue::warning("nested_composite_centering", message).with_values(
                        ref_offset.map_or("N/A".to_string(), |o| format!("{:.0}px", o)),
                        format!("{:.0}px", offset),
                    ),
                );
            }
        }
    }
}

/// Find parent-child relationships between composites by checking bounding box containment
/// Returns a map of child_id -> parent_id
fn find_composite_nesting(
    composites: &[CompositeState],
) -> std::collections::HashMap<String, String> {
    let mut nesting = std::collections::HashMap::new();

    for child in composites {
        // Find the smallest parent that contains this child
        let mut best_parent: Option<&CompositeState> = None;
        let mut best_parent_area = f64::MAX;

        for parent in composites {
            if parent.id == child.id {
                continue;
            }

            // Check if child is contained within parent
            let child_right = child.x + child.width;
            let child_bottom = child.y + child.height;
            let parent_right = parent.x + parent.width;
            let parent_bottom = parent.y + parent.height;

            // Child must be fully contained within parent (with small tolerance)
            let tolerance = 2.0;
            if child.x >= parent.x - tolerance
                && child.y >= parent.y - tolerance
                && child_right <= parent_right + tolerance
                && child_bottom <= parent_bottom + tolerance
            {
                let parent_area = parent.width * parent.height;
                if parent_area < best_parent_area {
                    best_parent = Some(parent);
                    best_parent_area = parent_area;
                }
            }
        }

        if let Some(parent) = best_parent {
            nesting.insert(child.id.clone(), parent.id.clone());
        }
    }

    nesting
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::Level;
    use crate::render::svg::structure::{ShapeCounts, ZOrderAnalysis};

    fn make_structure(nodes: usize, edges: usize, labels: Vec<&str>) -> SvgStructure {
        use crate::render::svg::structure::{
            ColorAnalysis, EdgeGeometry, FontAnalysis, StrokeAnalysis,
        };
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
            color_analysis: ColorAnalysis::default(),
            raw_svg: String::new(),
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
