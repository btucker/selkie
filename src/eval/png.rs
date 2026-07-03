//! PNG rendering and visual comparison for evaluation.
//!
//! This module provides SVG to PNG conversion and visual comparison
//! using SSIM (Structural Similarity Index).

#[cfg(feature = "png")]
use std::fs;
use std::path::Path;

/// RGBA image data with dimensions: (pixels, width, height)
#[cfg(feature = "png")]
pub type RgbaImage = (Vec<u8>, u32, u32);

#[cfg(feature = "png")]
use super::ssim::{calculate_ssim_registered, calculate_ssim_with_resize, rgba_to_grayscale};

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

/// Point resvg's generic `sans-serif` family at "Trebuchet MS".
///
/// Mermaid's stylesheet uses the lowercase family stack
/// `trebuchet ms, verdana, arial, sans-serif`. resvg's fontdb (0.23) matches
/// family names case-sensitively, so none of those lowercase names match the
/// installed "Trebuchet MS"/"Verdana"/"Arial" faces and resvg falls back to
/// whatever it resolves generic `sans-serif` to (typically Arial). Chrome —
/// which mmdc rasterizes the reference with — matches case-insensitively and
/// lands on the real Trebuchet MS. Aliasing the generic family here makes our
/// raster use the same glyph metrics Chrome does, so SSIM compares like with
/// like instead of penalizing an eval-only font substitution.
#[cfg(feature = "png")]
fn alias_sans_serif_to_trebuchet(opt: &mut resvg::usvg::Options) {
    opt.fontdb_mut().set_sans_serif_family("Trebuchet MS");
}

/// Convert SVG string to PNG bytes
#[cfg(feature = "png")]
pub fn svg_to_png(svg: &str) -> Result<(Vec<u8>, u32, u32), String> {
    use resvg::tiny_skia;
    use resvg::usvg;

    // Set up options with font database
    let mut opt = usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();
    alias_sans_serif_to_trebuchet(&mut opt);

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
    alias_sans_serif_to_trebuchet(&mut opt);

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

/// Get raw RGBA pixels from SVG, rendered scaled to a target pixel width.
///
/// Used to rasterize at the same scale as a reference raster (e.g. an mmdc
/// PNG limited by the puppeteer viewport) so SSIM compares two properly
/// anti-aliased images instead of point-sampling one down to the other.
#[cfg(feature = "png")]
pub fn svg_to_rgba_at_width(svg: &str, target_width: u32) -> Result<(Vec<u8>, u32, u32), String> {
    use resvg::tiny_skia;
    use resvg::usvg;

    // Set up options with font database
    let mut opt = usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();
    alias_sans_serif_to_trebuchet(&mut opt);

    // Parse SVG
    let tree =
        usvg::Tree::from_str(svg, &opt).map_err(|e| format!("Failed to parse SVG: {}", e))?;

    let size = tree.size();
    if size.width() <= 0.0 || size.height() <= 0.0 {
        return Err("SVG has zero intrinsic size".to_string());
    }

    let scale = target_width as f32 / size.width();
    let width = target_width.max(1);
    let height = ((size.height() * scale).round() as u32).max(1);

    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| "Failed to create pixmap".to_string())?;

    // Fill with white background
    pixmap.fill(tiny_skia::Color::WHITE);

    // Render scaled
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );

    Ok((pixmap.data().to_vec(), width, height))
}

/// Calculate visual similarity between a selkie SVG and the reference
/// rendered from mermaid source by mmdc (accurate text rendering).
///
/// The selkie SVG is rasterized at the reference PNG's pixel width so both
/// images are compared at a matched raster scale.
#[cfg(feature = "png")]
pub fn compare_svg_to_reference_source(
    selkie_svg: &str,
    reference_source: &str,
    cache: &super::cache::ReferenceCache,
) -> Result<VisualComparison, String> {
    let (reference_rgba, rw, rh) = sources_to_rgba_batch_cached(&[reference_source], cache)
        .into_iter()
        .next()
        .unwrap()?;

    let (selkie_rgba, sw, sh) = svg_to_rgba_at_width(selkie_svg, rw)?;

    let selkie_gray = rgba_to_grayscale(&selkie_rgba);
    let reference_gray = rgba_to_grayscale(&reference_rgba);
    let ssim = calculate_ssim_registered(&selkie_gray, sw, sh, &reference_gray, rw, rh);

    Ok(VisualComparison {
        ssim,
        selkie_dims: (sw, sh),
        reference_dims: (rw, rh),
    })
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

/// Render mermaid source to PNG using mmdc (for accurate text rendering)
#[cfg(feature = "png")]
pub fn source_to_rgba(source: &str) -> Result<(Vec<u8>, u32, u32), String> {
    // Use batch function for single diagram
    let results = sources_to_rgba_batch(&[source]);
    results.into_iter().next().unwrap()
}

/// Render multiple mermaid sources to PNGs in a single mmdc invocation, using cache
#[cfg(feature = "png")]
pub fn sources_to_rgba_batch_cached(
    sources: &[&str],
    cache: &super::cache::ReferenceCache,
) -> Vec<Result<RgbaImage, String>> {
    use image::GenericImageView;
    use std::process::{Command, Stdio};

    // Check which sources need rendering (not in cache)
    let uncached: Vec<(usize, &str)> = sources
        .iter()
        .enumerate()
        .filter(|(_, s)| !cache.is_png_cached(s))
        .map(|(i, s)| (i, *s))
        .collect();

    // If all cached, load from cache
    if uncached.is_empty() {
        return sources
            .iter()
            .map(|source| match cache.get_png(source) {
                Some(png_data) => match image::load_from_memory(&png_data) {
                    Ok(img) => {
                        let (width, height) = img.dimensions();
                        let rgba = img.into_rgba8().into_raw();
                        Ok((rgba, width, height))
                    }
                    Err(e) => Err(format!("Failed to decode cached PNG: {}", e)),
                },
                None => Err("Cache miss".to_string()),
            })
            .collect();
    }

    // Create temp directory
    let temp_dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(e) => {
            return sources
                .iter()
                .map(|_| Err(format!("Failed to create temp dir: {}", e)))
                .collect();
        }
    };

    // Create markdown file with only uncached diagrams
    let md_path = temp_dir.path().join("batch.md");
    let mut md_content = String::new();
    for (i, (_, source)) in uncached.iter().enumerate() {
        md_content.push_str(&format!(
            "## Diagram {}\n\n```mermaid\n{}\n```\n\n",
            i, source
        ));
    }

    if let Err(e) = fs::write(&md_path, &md_content) {
        return sources
            .iter()
            .map(|_| Err(format!("Failed to write markdown: {}", e)))
            .collect();
    }

    // Run mmdc with PNG output
    let output_md = temp_dir.path().join("output.md");
    let artefacts_dir = temp_dir.path().join("pngs");

    let output = Command::new("mmdc")
        .args([
            "-i",
            md_path.to_str().unwrap(),
            "-o",
            output_md.to_str().unwrap(),
            "-a",
            artefacts_dir.to_str().unwrap(),
            "-e",
            "png",
            "-q",
        ])
        .stderr(Stdio::piped())
        .output();

    let output = match output {
        Ok(o) => o,
        Err(e) => {
            return sources
                .iter()
                .map(|_| Err(format!("Failed to run mmdc: {}", e)))
                .collect();
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return sources
            .iter()
            .map(|_| Err(format!("mmdc failed: {}", stderr)))
            .collect();
    }

    // Read and cache rendered PNGs
    let batch_results: Vec<Result<(Vec<u8>, RgbaImage), String>> = uncached
        .iter()
        .enumerate()
        .map(|(i, (_, source))| {
            let png_path = artefacts_dir.join(format!("output-{}.png", i + 1));
            let png_data = fs::read(&png_path).map_err(|e| format!("Failed to read PNG: {}", e))?;

            // Cache the raw PNG data
            let _ = cache.put_png(source, &png_data);

            // Decode PNG to RGBA
            let img = image::load_from_memory(&png_data)
                .map_err(|e| format!("Failed to decode PNG: {}", e))?;

            let (width, height) = img.dimensions();
            let rgba = img.into_rgba8().into_raw();

            Ok((png_data, (rgba, width, height)))
        })
        .collect();

    // Build final results: batch results for uncached, cache for cached
    let mut batch_iter = batch_results.into_iter();
    let mut uncached_idx = 0;

    sources
        .iter()
        .enumerate()
        .map(|(i, source)| {
            if uncached_idx < uncached.len() && uncached[uncached_idx].0 == i {
                uncached_idx += 1;
                batch_iter.next().unwrap().map(|(_, rgba)| rgba)
            } else {
                // Load from cache
                match cache.get_png(source) {
                    Some(png_data) => match image::load_from_memory(&png_data) {
                        Ok(img) => {
                            let (width, height) = img.dimensions();
                            let rgba = img.into_rgba8().into_raw();
                            Ok((rgba, width, height))
                        }
                        Err(e) => Err(format!("Failed to decode cached PNG: {}", e)),
                    },
                    None => Err("Cache miss".to_string()),
                }
            }
        })
        .collect()
}

/// Render multiple mermaid sources to PNGs in a single mmdc invocation (no cache)
#[cfg(feature = "png")]
pub fn sources_to_rgba_batch(sources: &[&str]) -> Vec<Result<RgbaImage, String>> {
    use image::GenericImageView;
    use std::process::{Command, Stdio};

    // Create temp directory
    let temp_dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(e) => {
            return sources
                .iter()
                .map(|_| Err(format!("Failed to create temp dir: {}", e)))
                .collect();
        }
    };

    // Create markdown file with all diagrams as fenced code blocks
    let md_path = temp_dir.path().join("batch.md");
    let mut md_content = String::new();
    for (i, source) in sources.iter().enumerate() {
        md_content.push_str(&format!(
            "## Diagram {}\n\n```mermaid\n{}\n```\n\n",
            i, source
        ));
    }

    if let Err(e) = fs::write(&md_path, &md_content) {
        return sources
            .iter()
            .map(|_| Err(format!("Failed to write markdown: {}", e)))
            .collect();
    }

    // Run mmdc with PNG output
    let output_md = temp_dir.path().join("output.md");
    let artefacts_dir = temp_dir.path().join("pngs");

    let output = Command::new("mmdc")
        .args([
            "-i",
            md_path.to_str().unwrap(),
            "-o",
            output_md.to_str().unwrap(),
            "-a",
            artefacts_dir.to_str().unwrap(),
            "-e",
            "png",
            "-q",
        ])
        .stderr(Stdio::piped())
        .output();

    let output = match output {
        Ok(o) => o,
        Err(e) => {
            return sources
                .iter()
                .map(|_| Err(format!("Failed to run mmdc: {}", e)))
                .collect();
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return sources
            .iter()
            .map(|_| Err(format!("mmdc failed: {}", stderr)))
            .collect();
    }

    // Read generated PNGs (mmdc names them output-1.png, output-2.png, etc.)
    sources
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let png_path = artefacts_dir.join(format!("output-{}.png", i + 1));
            let png_data = fs::read(&png_path).map_err(|e| format!("Failed to read PNG: {}", e))?;

            // Decode PNG to RGBA
            let img = image::load_from_memory(&png_data)
                .map_err(|e| format!("Failed to decode PNG: {}", e))?;

            let (width, height) = img.dimensions();
            let rgba = img.into_rgba8().into_raw();

            Ok((rgba, width, height))
        })
        .collect()
}

/// Create a side-by-side comparison PNG
/// For reference images, uses mmdc to render from source for accurate text
#[cfg(feature = "png")]
pub fn create_comparison_png_with_source(
    selkie_svg: &str,
    reference_source: &str,
    _label: &str,
) -> Result<Vec<u8>, String> {
    use resvg::tiny_skia::{self, Color, Paint, Pixmap, Rect, Transform};

    // Render selkie SVG with resvg (our SVGs don't use foreignObject)
    let (selkie_rgba, sw, sh) = svg_to_rgba(selkie_svg)?;

    // Render reference from source with mmdc (handles foreignObject text)
    let (reference_rgba, rw, rh) = source_to_rgba(reference_source)?;

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

    // Draw divider
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

/// Create a side-by-side comparison PNG from pre-rendered RGBA data
#[cfg(feature = "png")]
pub fn create_comparison_png_from_rgba(
    selkie_rgba: &[u8],
    sw: u32,
    sh: u32,
    reference_rgba: &[u8],
    rw: u32,
    rh: u32,
) -> Result<Vec<u8>, String> {
    use resvg::tiny_skia::{Color, Paint, Pixmap, Rect, Transform};

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
        pm.data_mut().copy_from_slice(selkie_rgba);
        pm
    };

    let reference_pixmap = {
        let mut pm = Pixmap::new(rw, rh).ok_or("Failed to create reference pixmap")?;
        pm.data_mut().copy_from_slice(reference_rgba);
        pm
    };

    // Draw selkie image on left
    let selkie_x = padding as i32;
    let selkie_y = padding as i32;
    combined.draw_pixmap(
        selkie_x,
        selkie_y,
        selkie_pixmap.as_ref(),
        &resvg::tiny_skia::PixmapPaint::default(),
        Transform::identity(),
        None,
    );

    // Draw divider
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
        &resvg::tiny_skia::PixmapPaint::default(),
        Transform::identity(),
        None,
    );

    // Encode to PNG
    let png_data = combined
        .encode_png()
        .map_err(|e| format!("Failed to encode PNG: {}", e))?;

    Ok(png_data)
}

/// Create a side-by-side comparison PNG (legacy - both from SVG)
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

/// Write comparison PNGs to subdirectories organized by diagram type
#[cfg(feature = "png")]
pub fn write_comparison_pngs(
    output_dir: &Path,
    comparisons: &[(String, String, String, String, String)], // (name, diagram_type, source_text, selkie_svg, reference_svg)
    cache: &super::cache::ReferenceCache,
) -> Result<super::report::PngManifest, String> {
    use super::report::{PngManifest, PngManifestEntry};

    let mut manifest = PngManifest {
        diagrams: Vec::new(),
    };

    // Batch render all reference PNGs with mmdc (uses cache)
    let source_texts: Vec<&str> = comparisons
        .iter()
        .map(|(_, _, s, _, _)| s.as_str())
        .collect();

    // Count cached vs uncached for progress display
    let cached_count = source_texts
        .iter()
        .filter(|s| cache.is_png_cached(s))
        .count();
    if cached_count == source_texts.len() {
        eprint!(" ({} cached)...", cached_count);
    } else {
        eprint!(
            " rendering {} references ({} cached)...",
            source_texts.len() - cached_count,
            cached_count
        );
    }

    let reference_rgbas = sources_to_rgba_batch_cached(&source_texts, cache);

    // Render all selkie SVGs to RGBA (fast, uses resvg in-process).
    // Render at the reference PNG's pixel width so SSIM compares two
    // anti-aliased rasters at a matched scale.
    eprint!(" rendering selkie...");
    let selkie_rgbas: Vec<Result<RgbaImage, String>> = comparisons
        .iter()
        .enumerate()
        .map(|(i, (_, _, _, selkie_svg, _))| match &reference_rgbas[i] {
            Ok((_, rw, _)) => svg_to_rgba_at_width(selkie_svg, *rw),
            Err(_) => svg_to_rgba(selkie_svg),
        })
        .collect();

    eprint!(" compositing...");

    // Process each comparison
    for (i, (name, diagram_type, _source_text, _selkie_svg, _reference_svg)) in
        comparisons.iter().enumerate()
    {
        // Create subdirectory for this diagram type
        let type_dir = output_dir.join(diagram_type);
        fs::create_dir_all(&type_dir)
            .map_err(|e| format!("Failed to create directory {}: {}", type_dir.display(), e))?;

        // Get pre-rendered RGBA data
        let (selkie_rgba, sw, sh) = match &selkie_rgbas[i] {
            Ok((rgba, w, h)) => (rgba.as_slice(), *w, *h),
            Err(e) => {
                eprintln!("Warning: Failed to render selkie SVG for {}: {}", name, e);
                continue;
            }
        };

        let (reference_rgba, rw, rh) = match &reference_rgbas[i] {
            Ok((rgba, w, h)) => (rgba.as_slice(), *w, *h),
            Err(e) => {
                eprintln!(
                    "Warning: Failed to render reference PNG for {}: {}",
                    name, e
                );
                continue;
            }
        };

        // Calculate visual similarity between renderings
        let selkie_gray = rgba_to_grayscale(selkie_rgba);
        let reference_gray = rgba_to_grayscale(reference_rgba);
        let ssim = calculate_ssim_registered(&selkie_gray, sw, sh, &reference_gray, rw, rh);

        // Create side-by-side comparison PNG from pre-rendered RGBA data
        let png_data =
            create_comparison_png_from_rgba(selkie_rgba, sw, sh, reference_rgba, rw, rh)?;

        // Write PNG file to type subdirectory
        let safe_name = name.replace(['/', ' '], "_");
        let png_filename = format!("{}_comparison.png", safe_name);
        let png_path = type_dir.join(&png_filename);
        fs::write(&png_path, &png_data)
            .map_err(|e| format!("Failed to write PNG {}: {}", png_path.display(), e))?;

        // Add to manifest with relative path including type directory
        manifest.diagrams.push(PngManifestEntry {
            name: name.clone(),
            diagram_type: diagram_type.clone(),
            png: format!("{}/{}", diagram_type, png_filename),
            structural_match: true, // Will be filled in by caller
            visual_similarity: Some(ssim),
            issues: Vec::new(), // Will be filled in by caller
        });
    }

    // Write manifest to root output directory
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
pub fn compare_svg_to_reference_source(
    _selkie_svg: &str,
    _reference_source: &str,
    _cache: &super::cache::ReferenceCache,
) -> Result<VisualComparison, String> {
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
    _comparisons: &[(String, String, String, String, String)], // (name, diagram_type, source_text, selkie_svg, reference_svg)
    _cache: &super::cache::ReferenceCache,
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
    fn test_svg_to_rgba_at_width_scales_to_target() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80">
            <rect x="10" y="10" width="80" height="60" fill="blue"/>
        </svg>"#;

        let (data, w, h) = svg_to_rgba_at_width(svg, 50).unwrap();
        assert_eq!(w, 50);
        assert_eq!(h, 40);
        assert_eq!(data.len(), (50 * 40 * 4) as usize);
    }

    #[test]
    #[cfg(feature = "png")]
    fn test_matched_scale_ssim_high_for_identical_content() {
        use super::super::ssim::calculate_ssim_with_resize;

        // Fine 1px strokes alias badly when one raster is point-sampled down
        // to the other's size. Rendering at matched scale (anti-aliased) plus
        // box-filter resizing must keep SSIM near 1.0 for identical content.
        let mut svg =
            String::from(r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200">"#);
        for i in 0..40 {
            svg.push_str(&format!(
                r#"<line x1="0" y1="{y}" x2="200" y2="{y}" stroke="black" stroke-width="1"/>"#,
                y = i * 5
            ));
        }
        svg.push_str("</svg>");

        // Intrinsic-size render vs a render simulating the reference raster
        // scale (e.g. mmdc's ~784px viewport vs a wider SVG).
        let (a_rgba, aw, ah) = svg_to_rgba(&svg).unwrap();
        let (b_rgba, bw, bh) = svg_to_rgba_at_width(&svg, 117).unwrap();

        let a_gray = rgba_to_grayscale(&a_rgba);
        let b_gray = rgba_to_grayscale(&b_rgba);
        let ssim = calculate_ssim_with_resize(&a_gray, aw, ah, &b_gray, bw, bh);

        assert!(
            ssim > 0.9,
            "identical content at different raster scales must score near 1.0, got {ssim}"
        );
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
