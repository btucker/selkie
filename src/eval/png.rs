//! PNG rendering and visual comparison for evaluation.
//!
//! This module provides SVG to PNG conversion and visual comparison
//! using SSIM (Structural Similarity Index).

#[cfg(feature = "png")]
use std::fs;
use std::path::Path;

#[cfg(feature = "png")]
use super::ssim::{calculate_ssim_with_resize, rgba_to_grayscale};

/// Result of visual comparison between two images
#[derive(Debug, Clone)]
pub struct VisualComparison {
    /// SSIM score (0-1, where 1 is identical)
    pub ssim: f64,
    /// Selkie image dimensions
    pub selkie_dims: (u32, u32),
    /// Reference image dimensions
    pub reference_dims: (u32, u32),
}

/// Convert SVG string to PNG bytes
#[cfg(feature = "png")]
pub fn svg_to_png(svg: &str) -> Result<(Vec<u8>, u32, u32), String> {
    use resvg::tiny_skia;
    use resvg::usvg;

    // Set up options with font database
    let mut opt = usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();

    // Parse SVG
    let tree =
        usvg::Tree::from_str(svg, &opt).map_err(|e| format!("Failed to parse SVG: {}", e))?;

    // Get the size
    let size = tree.size();
    let width = size.width().ceil() as u32;
    let height = size.height().ceil() as u32;

    // Create pixmap
    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| "Failed to create pixmap".to_string())?;

    // Fill with white background
    pixmap.fill(tiny_skia::Color::WHITE);

    // Render
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );

    // Encode to PNG
    let png_data = pixmap
        .encode_png()
        .map_err(|e| format!("Failed to encode PNG: {}", e))?;

    Ok((png_data, width, height))
}

/// Get raw RGBA pixels from SVG
#[cfg(feature = "png")]
pub fn svg_to_rgba(svg: &str) -> Result<(Vec<u8>, u32, u32), String> {
    use resvg::tiny_skia;
    use resvg::usvg;

    // Set up options with font database
    let mut opt = usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();

    // Parse SVG
    let tree =
        usvg::Tree::from_str(svg, &opt).map_err(|e| format!("Failed to parse SVG: {}", e))?;

    // Get the size
    let size = tree.size();
    let width = size.width().ceil() as u32;
    let height = size.height().ceil() as u32;

    // Create pixmap
    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| "Failed to create pixmap".to_string())?;

    // Fill with white background
    pixmap.fill(tiny_skia::Color::WHITE);

    // Render
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );

    // Get RGBA data
    Ok((pixmap.data().to_vec(), width, height))
}

/// Calculate visual similarity between two SVGs
#[cfg(feature = "png")]
pub fn compare_svgs(selkie_svg: &str, reference_svg: &str) -> Result<VisualComparison, String> {
    // Render both to RGBA
    let (selkie_rgba, sw, sh) = svg_to_rgba(selkie_svg)?;
    let (reference_rgba, rw, rh) = svg_to_rgba(reference_svg)?;

    // Convert to grayscale for SSIM
    let selkie_gray = rgba_to_grayscale(&selkie_rgba);
    let reference_gray = rgba_to_grayscale(&reference_rgba);

    // Calculate SSIM (handles different sizes)
    let ssim = calculate_ssim_with_resize(&selkie_gray, sw, sh, &reference_gray, rw, rh);

    Ok(VisualComparison {
        ssim,
        selkie_dims: (sw, sh),
        reference_dims: (rw, rh),
    })
}

/// Create a side-by-side comparison PNG
#[cfg(feature = "png")]
pub fn create_comparison_png(
    selkie_svg: &str,
    reference_svg: &str,
    label: &str,
) -> Result<Vec<u8>, String> {
    use resvg::tiny_skia::{self, Color, Paint, Pixmap, Rect, Transform};

    let _ = label; // Not used for now (could add text labels later)

    // Render both SVGs
    let (selkie_rgba, sw, sh) = svg_to_rgba(selkie_svg)?;
    let (reference_rgba, rw, rh) = svg_to_rgba(reference_svg)?;

    // Calculate combined dimensions
    // Layout: [Selkie] | [Divider] | [Reference]
    let divider_width = 4u32;
    let padding = 10u32;

    let max_height = sh.max(rh);
    let total_width = sw + rw + divider_width + padding * 4;
    let total_height = max_height + padding * 2;

    // Create combined pixmap
    let mut combined = Pixmap::new(total_width, total_height)
        .ok_or_else(|| "Failed to create combined pixmap".to_string())?;

    // Fill with light gray background
    combined.fill(Color::from_rgba8(245, 245, 245, 255));

    // Create pixmaps from RGBA data
    let selkie_pixmap = {
        let mut pm = Pixmap::new(sw, sh).ok_or("Failed to create selkie pixmap")?;
        pm.data_mut().copy_from_slice(&selkie_rgba);
        pm
    };

    let reference_pixmap = {
        let mut pm = Pixmap::new(rw, rh).ok_or("Failed to create reference pixmap")?;
        pm.data_mut().copy_from_slice(&reference_rgba);
        pm
    };

    // Draw selkie image on left
    let selkie_x = padding as i32;
    let selkie_y = padding as i32;
    combined.draw_pixmap(
        selkie_x,
        selkie_y,
        selkie_pixmap.as_ref(),
        &tiny_skia::PixmapPaint::default(),
        Transform::identity(),
        None,
    );

    // Draw divider using fill_rect
    let divider_x = (sw + padding * 2) as f32;
    let divider_rect = Rect::from_xywh(
        divider_x,
        padding as f32,
        divider_width as f32,
        max_height as f32,
    );
    if let Some(rect) = divider_rect {
        let mut paint = Paint::default();
        paint.set_color_rgba8(100, 100, 100, 255);
        combined.fill_rect(rect, &paint, Transform::identity(), None);
    }

    // Draw reference image on right
    let reference_x = (sw + divider_width + padding * 3) as i32;
    let reference_y = padding as i32;
    combined.draw_pixmap(
        reference_x,
        reference_y,
        reference_pixmap.as_ref(),
        &tiny_skia::PixmapPaint::default(),
        Transform::identity(),
        None,
    );

    // Encode to PNG
    let png_data = combined
        .encode_png()
        .map_err(|e| format!("Failed to encode PNG: {}", e))?;

    Ok(png_data)
}

/// Write comparison PNGs and manifest to directory
#[cfg(feature = "png")]
pub fn write_comparison_pngs(
    output_dir: &Path,
    comparisons: &[(String, String, String)], // (name, selkie_svg, reference_svg)
) -> Result<super::report::PngManifest, String> {
    use super::report::{PngManifest, PngManifestEntry};

    // Create output directory
    fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    let mut manifest = PngManifest {
        diagrams: Vec::new(),
    };

    for (name, selkie_svg, reference_svg) in comparisons {
        // Calculate visual similarity
        let comparison = compare_svgs(selkie_svg, reference_svg)?;

        // Create comparison PNG
        let png_data = create_comparison_png(selkie_svg, reference_svg, name)?;

        // Write PNG file
        let png_filename = format!("{}.png", name.replace('/', "_").replace(' ', "_"));
        let png_path = output_dir.join(&png_filename);
        fs::write(&png_path, &png_data)
            .map_err(|e| format!("Failed to write PNG {}: {}", png_path.display(), e))?;

        // Add to manifest
        manifest.diagrams.push(PngManifestEntry {
            name: name.clone(),
            diagram_type: String::new(), // Will be filled in by caller
            png: png_filename,
            structural_match: true, // Will be filled in by caller
            visual_similarity: Some(comparison.ssim),
            issues: Vec::new(), // Will be filled in by caller
        });
    }

    // Write manifest
    let manifest_path = output_dir.join("manifest.json");
    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
    fs::write(&manifest_path, manifest_json)
        .map_err(|e| format!("Failed to write manifest: {}", e))?;

    Ok(manifest)
}

// Stub implementations when png feature is not enabled
#[cfg(not(feature = "png"))]
pub fn svg_to_png(_svg: &str) -> Result<(Vec<u8>, u32, u32), String> {
    Err("PNG feature not enabled. Build with --features png".to_string())
}

#[cfg(not(feature = "png"))]
pub fn svg_to_rgba(_svg: &str) -> Result<(Vec<u8>, u32, u32), String> {
    Err("PNG feature not enabled. Build with --features png".to_string())
}

#[cfg(not(feature = "png"))]
pub fn compare_svgs(_selkie_svg: &str, _reference_svg: &str) -> Result<VisualComparison, String> {
    Err("PNG feature not enabled. Build with --features png".to_string())
}

#[cfg(not(feature = "png"))]
pub fn create_comparison_png(
    _selkie_svg: &str,
    _reference_svg: &str,
    _label: &str,
) -> Result<Vec<u8>, String> {
    Err("PNG feature not enabled. Build with --features png".to_string())
}

#[cfg(not(feature = "png"))]
pub fn write_comparison_pngs(
    _output_dir: &Path,
    _comparisons: &[(String, String, String)],
) -> Result<super::report::PngManifest, String> {
    Err("PNG feature not enabled. Build with --features png".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "png")]
    fn test_svg_to_png() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <rect x="10" y="10" width="80" height="80" fill="blue"/>
        </svg>"#;

        let result = svg_to_png(svg);
        assert!(result.is_ok());

        let (data, w, h) = result.unwrap();
        assert!(!data.is_empty());
        assert_eq!(w, 100);
        assert_eq!(h, 100);
    }

    #[test]
    #[cfg(feature = "png")]
    fn test_compare_identical_svgs() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <rect x="10" y="10" width="80" height="80" fill="blue"/>
        </svg>"#;

        let result = compare_svgs(svg, svg);
        assert!(result.is_ok());

        let comparison = result.unwrap();
        assert!(
            comparison.ssim > 0.99,
            "Identical SVGs should have SSIM > 0.99"
        );
    }

    #[test]
    #[cfg(feature = "png")]
    fn test_svg_with_foreign_object_should_succeed() {
        // Mermaid.js uses foreignObject with HTML content for labels
        // resvg cannot parse HTML p tags, so we need to pre-process
        let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"200\" height=\"100\">\
            <rect x=\"10\" y=\"10\" width=\"180\" height=\"80\" fill=\"blue\"/>\
            <foreignObject x=\"20\" y=\"30\" width=\"160\" height=\"40\">\
            <div xmlns=\"http://www.w3.org/1999/xhtml\">\
            <p>Hello World</p>\
            </div>\
            </foreignObject>\
            </svg>";

        // Should succeed after pre-processing strips foreignObject
        let result = svg_to_png(svg);
        assert!(result.is_ok(), "SVG with foreignObject should render");
    }

    #[test]
    #[cfg(feature = "png")]
    fn test_svg_with_mermaid_style_foreign_object() {
        // Exact structure used by mermaid.js flowcharts
        let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"200\" height=\"100\">\
            <g class=\"nodes\">\
            <g class=\"node\" transform=\"translate(100, 50)\">\
            <rect x=\"-40\" y=\"-20\" width=\"80\" height=\"40\"/>\
            <g class=\"label\">\
            <rect></rect>\
            <foreignObject width=\"60\" height=\"24\">\
            <div xmlns=\"http://www.w3.org/1999/xhtml\" style=\"display: table-cell;\">\
            <span class=\"nodeLabel\"><p>Start</p></span>\
            </div>\
            </foreignObject>\
            </g>\
            </g>\
            </g>\
            </svg>";

        let result = svg_to_png(svg);
        assert!(result.is_ok(), "Mermaid-style foreignObject should render");
    }

    #[test]
    #[cfg(feature = "png")]
    fn test_real_mermaid_flowchart_svg() {
        // An actual mermaid.js flowchart SVG with foreignObject HTML
        let svg = include_str!("../../tests/fixtures/mermaid_flowchart.svg");
        let result = svg_to_png(svg);
        // resvg handles foreignObject with HTML content
        assert!(result.is_ok(), "Real mermaid SVG should render to PNG");
        let (data, w, h) = result.unwrap();
        assert!(!data.is_empty(), "PNG should have data");
        assert!(w > 0 && h > 0, "PNG should have dimensions");
    }
}
