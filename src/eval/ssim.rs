//! SSIM (Structural Similarity Index) calculation for visual comparison.
//!
//! SSIM is a perceptual metric that measures the similarity between two images.
//! It returns a value between 0 and 1, where 1 means identical images.
//!
//! Reference: Wang, Z., Bovik, A. C., Sheikh, H. R., & Simoncelli, E. P. (2004).
//! "Image quality assessment: from error visibility to structural similarity"

/// SSIM constants (default values from the original paper)
const K1: f64 = 0.01;
const K2: f64 = 0.03;
const L: f64 = 255.0; // Dynamic range for 8-bit images

/// Local window size for SSIM (8x8 as in the original paper's block variant)
const WINDOW: u32 = 8;
/// Stride between windows (overlapping windows, half-window step)
const STRIDE: u32 = 4;

/// Calculate mean SSIM between two images represented as grayscale pixel arrays
///
/// Follows Wang et al. (2004): SSIM is computed over local windows and the
/// per-window scores are averaged. A single global statistic is NOT SSIM and
/// collapses when images differ in a localized region, understating
/// similarity of otherwise identical renders.
///
/// Both images must have the same dimensions.
/// Returns a value between 0 and 1 (1 = identical).
pub fn calculate_ssim(img1: &[u8], img2: &[u8], width: u32, height: u32) -> f64 {
    if img1.len() != img2.len() || img1.len() != (width as usize) * (height as usize) {
        return 0.0;
    }
    if img1.is_empty() {
        return 1.0;
    }

    // Images smaller than the window get a single window over the whole image
    let win_w = WINDOW.min(width);
    let win_h = WINDOW.min(height);

    let mut sum = 0.0;
    let mut count = 0usize;

    let mut y = 0;
    loop {
        let y_end = (y + win_h).min(height);
        let y_start = y_end.saturating_sub(win_h);
        let mut x = 0;
        loop {
            let x_end = (x + win_w).min(width);
            let x_start = x_end.saturating_sub(win_w);

            sum += window_ssim(img1, img2, width, x_start, y_start, x_end, y_end);
            count += 1;

            if x_end >= width {
                break;
            }
            x += STRIDE;
        }
        if y_end >= height {
            break;
        }
        y += STRIDE;
    }

    if count == 0 {
        return 1.0;
    }
    sum / count as f64
}

/// SSIM statistic for a single window [x_start, x_end) x [y_start, y_end)
fn window_ssim(
    img1: &[u8],
    img2: &[u8],
    width: u32,
    x_start: u32,
    y_start: u32,
    x_end: u32,
    y_end: u32,
) -> f64 {
    let n = ((x_end - x_start) * (y_end - y_start)) as f64;
    if n == 0.0 {
        return 1.0;
    }

    let mut sum1 = 0.0;
    let mut sum2 = 0.0;
    let mut sum1_sq = 0.0;
    let mut sum2_sq = 0.0;
    let mut sum12 = 0.0;

    for y in y_start..y_end {
        let row = (y * width) as usize;
        for x in x_start..x_end {
            let p1 = img1[row + x as usize] as f64;
            let p2 = img2[row + x as usize] as f64;
            sum1 += p1;
            sum2 += p2;
            sum1_sq += p1 * p1;
            sum2_sq += p2 * p2;
            sum12 += p1 * p2;
        }
    }

    let mean1 = sum1 / n;
    let mean2 = sum2 / n;
    let var1 = (sum1_sq / n - mean1 * mean1).max(0.0);
    let var2 = (sum2_sq / n - mean2 * mean2).max(0.0);
    let covar = sum12 / n - mean1 * mean2;

    let c1 = (K1 * L).powi(2);
    let c2 = (K2 * L).powi(2);

    let numerator = (2.0 * mean1 * mean2 + c1) * (2.0 * covar + c2);
    let denominator = (mean1.powi(2) + mean2.powi(2) + c1) * (var1 + var2 + c2);

    if denominator == 0.0 {
        return 1.0;
    }

    numerator / denominator
}

/// Convert RGBA pixels to grayscale using luminance formula
pub fn rgba_to_grayscale(rgba: &[u8]) -> Vec<u8> {
    rgba.chunks(4)
        .map(|pixel| {
            let r = pixel[0] as f64;
            let g = pixel[1] as f64;
            let b = pixel[2] as f64;
            // ITU-R BT.601 luma coefficients
            (0.299 * r + 0.587 * g + 0.114 * b) as u8
        })
        .collect()
}

/// Calculate SSIM between two RGBA images
///
/// Converts to grayscale internally and computes SSIM.
/// Both images must have the same dimensions.
pub fn calculate_ssim_rgba(img1_rgba: &[u8], img2_rgba: &[u8], width: u32, height: u32) -> f64 {
    let gray1 = rgba_to_grayscale(img1_rgba);
    let gray2 = rgba_to_grayscale(img2_rgba);
    calculate_ssim(&gray1, &gray2, width, height)
}

/// Resize image to target dimensions using area-averaging (box filter)
///
/// Each destination pixel is the area-weighted average of the source pixels
/// it covers. Unlike nearest-neighbor point sampling, this preserves thin
/// (1px) strokes as gray instead of dropping them, which keeps SSIM honest
/// when comparing rasters produced at different scales.
pub fn resize_grayscale(
    src: &[u8],
    src_width: u32,
    src_height: u32,
    dst_width: u32,
    dst_height: u32,
) -> Vec<u8> {
    let mut dst = vec![0u8; (dst_width as usize) * (dst_height as usize)];
    if src_width == 0 || src_height == 0 || dst_width == 0 || dst_height == 0 {
        return dst;
    }

    let x_ratio = src_width as f64 / dst_width as f64;
    let y_ratio = src_height as f64 / dst_height as f64;

    for y in 0..dst_height {
        // Source row span covered by this destination row
        let y0 = y as f64 * y_ratio;
        let y1 = (y as f64 + 1.0) * y_ratio;
        let sy_start = y0.floor() as u32;
        let sy_end = (y1.ceil() as u32).min(src_height);

        for x in 0..dst_width {
            // Source column span covered by this destination column
            let x0 = x as f64 * x_ratio;
            let x1 = (x as f64 + 1.0) * x_ratio;
            let sx_start = x0.floor() as u32;
            let sx_end = (x1.ceil() as u32).min(src_width);

            let mut sum = 0.0;
            let mut area = 0.0;

            for sy in sy_start..sy_end {
                // Overlap of source row sy with [y0, y1)
                let wy = (y1.min(sy as f64 + 1.0) - y0.max(sy as f64)).max(0.0);
                if wy == 0.0 {
                    continue;
                }
                let row_base = (sy * src_width) as usize;
                for sx in sx_start..sx_end {
                    // Overlap of source column sx with [x0, x1)
                    let wx = (x1.min(sx as f64 + 1.0) - x0.max(sx as f64)).max(0.0);
                    let weight = wx * wy;
                    if weight > 0.0 {
                        if let Some(&pixel) = src.get(row_base + sx as usize) {
                            sum += pixel as f64 * weight;
                            area += weight;
                        }
                    }
                }
            }

            let dst_idx = (y * dst_width + x) as usize;
            dst[dst_idx] = if area > 0.0 {
                (sum / area).round().clamp(0.0, 255.0) as u8
            } else {
                0
            };
        }
    }

    dst
}

/// Calculate SSIM between two images that may have different dimensions
///
/// If dimensions differ, the larger image is resized down to match the smaller one.
pub fn calculate_ssim_with_resize(
    img1: &[u8],
    w1: u32,
    h1: u32,
    img2: &[u8],
    w2: u32,
    h2: u32,
) -> f64 {
    if w1 == w2 && h1 == h2 {
        return calculate_ssim(img1, img2, w1, h1);
    }

    // Use smaller dimensions as target
    let target_w = w1.min(w2);
    let target_h = h1.min(h2);

    let resized1 = if w1 != target_w || h1 != target_h {
        resize_grayscale(img1, w1, h1, target_w, target_h)
    } else {
        img1.to_vec()
    };

    let resized2 = if w2 != target_w || h2 != target_h {
        resize_grayscale(img2, w2, h2, target_w, target_h)
    } else {
        img2.to_vec()
    };

    calculate_ssim(&resized1, &resized2, target_w, target_h)
}

/// Luma threshold below which a pixel counts as content rather than the white
/// background. mmdc/Chrome reference PNGs and selkie rasters are both composited
/// on white; anti-aliased edges are darker than this.
const CONTENT_THRESHOLD: u8 = 250;

/// Minimum long-edge (in pixels) at which the registered content boxes are
/// compared. Windowed SSIM deflates monotonically as the raster shrinks: the
/// fixed 8x8 window covers a larger fraction of a tiny image, so sub-pixel and
/// anti-aliasing differences decorrelate nearly every window and a minimal 2-4
/// node diagram (~110-160px on the long edge at mmdc scale 1) is systematically
/// under-scored independent of render quality. Below this floor both registered
/// crops are upsampled to a genuine resolution before windowing so the score
/// reflects render quality rather than raster area. Upsampling adds no new
/// information, so it cannot mask a real geometric divergence — mismatched
/// regions stay mismatched at every scale; it only stops the window/image size
/// ratio from deflating an otherwise-faithful tiny render.
const MIN_REGISTERED_LONG_EDGE: u32 = 700;

/// Maximum tolerated ratio between the two images' content aspect ratios before
/// registration is skipped. Layout-aligned diagrams differ only by white slack
/// and sub-pixel scale, so their content boxes share an aspect ratio; a larger
/// mismatch means the layouts genuinely differ and must NOT be masked by
/// cropping and rescaling into alignment.
const AR_TOLERANCE: f64 = 1.10;

/// Non-white content bounding box as (x0, y0, x1, y1) with exclusive ends,
/// or None if the image is entirely background.
fn content_bbox(gray: &[u8], width: u32, height: u32) -> Option<(u32, u32, u32, u32)> {
    if gray.len() != (width as usize) * (height as usize) {
        return None;
    }
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    let mut found = false;
    for y in 0..height {
        let row = (y * width) as usize;
        for x in 0..width {
            if gray[row + x as usize] < CONTENT_THRESHOLD {
                found = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    if found {
        Some((min_x, min_y, max_x + 1, max_y + 1))
    } else {
        None
    }
}

/// Copy the sub-rectangle `bbox` out of `gray` into a tightly packed buffer.
fn crop_grayscale(gray: &[u8], width: u32, bbox: (u32, u32, u32, u32)) -> (Vec<u8>, u32, u32) {
    let (x0, y0, x1, y1) = bbox;
    let cw = x1 - x0;
    let ch = y1 - y0;
    let mut out = vec![0u8; (cw as usize) * (ch as usize)];
    for y in 0..ch {
        let src = (((y0 + y) * width) + x0) as usize;
        let dst = (y * cw) as usize;
        out[dst..dst + cw as usize].copy_from_slice(&gray[src..src + cw as usize]);
    }
    (out, cw, ch)
}

/// Registration-corrected SSIM.
///
/// mmdc/Chrome reference PNGs pad the diagram with white slack, so the same
/// diagram fills a slightly different pixel fraction than selkie's raster. Even
/// a ~1% scale mismatch plus a translation decorrelates the overlapping SSIM
/// windows progressively toward the far edge, understating similarity of an
/// otherwise identical layout. This aligns the two images by their non-white
/// content bounding boxes (removing translation) and rescales the boxes to
/// common dimensions (removing scale) before computing SSIM.
///
/// When the two content boxes' aspect ratios diverge beyond [`AR_TOLERANCE`],
/// the layouts genuinely differ; registration is skipped and the plain
/// resize-based compare is used so the divergence is not masked.
pub fn calculate_ssim_registered(
    img1: &[u8],
    w1: u32,
    h1: u32,
    img2: &[u8],
    w2: u32,
    h2: u32,
) -> f64 {
    let (b1, b2) = match (content_bbox(img1, w1, h1), content_bbox(img2, w2, h2)) {
        (Some(a), Some(b)) => (a, b),
        // One or both blank: nothing to register against.
        _ => return calculate_ssim_with_resize(img1, w1, h1, img2, w2, h2),
    };

    let (c1, cw1, ch1) = crop_grayscale(img1, w1, b1);
    let (c2, cw2, ch2) = crop_grayscale(img2, w2, b2);

    let ar1 = cw1 as f64 / ch1 as f64;
    let ar2 = cw2 as f64 / ch2 as f64;
    if (ar1 / ar2).max(ar2 / ar1) > AR_TOLERANCE {
        return calculate_ssim_with_resize(img1, w1, h1, img2, w2, h2);
    }

    // Rescale both content boxes to common dimensions so residual scale
    // differences are removed before comparison. The base common size is the
    // smaller of each axis; the minimum render-scale floor then raises it so a
    // tiny diagram is compared at a genuine resolution instead of being
    // deflated by the fixed window covering a large fraction of a small raster.
    let base_w = cw1.min(cw2).max(1);
    let base_h = ch1.min(ch2).max(1);
    let (target_w, target_h) = floor_dims(base_w, base_h);

    let r1 = if cw1 != target_w || ch1 != target_h {
        resize_grayscale(&c1, cw1, ch1, target_w, target_h)
    } else {
        c1
    };
    let r2 = if cw2 != target_w || ch2 != target_h {
        resize_grayscale(&c2, cw2, ch2, target_w, target_h)
    } else {
        c2
    };

    calculate_ssim(&r1, &r2, target_w, target_h)
}

/// Scale `(w, h)` up so its long edge reaches [`MIN_REGISTERED_LONG_EDGE`],
/// preserving aspect ratio. Returns the input unchanged when it already meets
/// (or exceeds) the floor.
fn floor_dims(w: u32, h: u32) -> (u32, u32) {
    let long = w.max(h);
    if long == 0 || long >= MIN_REGISTERED_LONG_EDGE {
        return (w, h);
    }
    let scale = MIN_REGISTERED_LONG_EDGE as f64 / long as f64;
    let fw = ((w as f64 * scale).round() as u32).max(1);
    let fh = ((h as f64 * scale).round() as u32).max(1);
    (fw, fh)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_images() {
        let img = vec![100u8; 100];
        let ssim = calculate_ssim(&img, &img, 10, 10);
        assert!(
            (ssim - 1.0).abs() < 0.001,
            "Identical images should have SSIM ~1.0"
        );
    }

    #[test]
    fn test_completely_different() {
        let img1 = vec![0u8; 100];
        let img2 = vec![255u8; 100];
        let ssim = calculate_ssim(&img1, &img2, 10, 10);
        assert!(
            ssim < 0.1,
            "Completely different images should have low SSIM"
        );
    }

    #[test]
    fn test_similar_images() {
        let img1: Vec<u8> = (0..100).map(|i| (i * 2) as u8).collect();
        let img2: Vec<u8> = (0..100).map(|i| (i * 2 + 5) as u8).collect();
        let ssim = calculate_ssim(&img1, &img2, 10, 10);
        assert!(ssim > 0.9, "Similar images should have high SSIM");
    }

    #[test]
    fn test_localized_difference_does_not_collapse_score() {
        // Wang et al. SSIM is computed over local windows and averaged.
        // A single global-covariance statistic collapses when one image has
        // a localized feature the other lacks, even though the images are
        // 98% identical. Windowed SSIM must score this pair high.
        let w = 64u32;
        let h = 64u32;
        let img1 = vec![255u8; (w * h) as usize];
        let mut img2 = img1.clone();
        // 8x8 black square in one corner (~1.5% of the image)
        for y in 0..8 {
            for x in 0..8 {
                img2[(y * w + x) as usize] = 0;
            }
        }

        let ssim = calculate_ssim(&img1, &img2, w, h);
        assert!(
            ssim > 0.7,
            "images identical except a tiny patch must score high, got {ssim}"
        );
    }

    #[test]
    fn test_rgba_to_grayscale() {
        let rgba = vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255];
        let gray = rgba_to_grayscale(&rgba);
        assert_eq!(gray.len(), 3);
        // Red: 0.299 * 255 ≈ 76
        // Green: 0.587 * 255 ≈ 150
        // Blue: 0.114 * 255 ≈ 29
        assert!((gray[0] as i32 - 76).abs() < 2);
        assert!((gray[1] as i32 - 150).abs() < 2);
        assert!((gray[2] as i32 - 29).abs() < 2);
    }

    #[test]
    fn test_resize() {
        let src = vec![0, 1, 2, 3, 4, 5, 6, 7, 8];
        let resized = resize_grayscale(&src, 3, 3, 2, 2);
        assert_eq!(resized.len(), 4);
    }

    #[test]
    fn test_resize_averages_source_pixels() {
        // 4x4 image with alternating 0/255 columns. A 2x downscale must
        // average neighboring pixels (box filter) instead of point sampling,
        // which would drop every 255 column and return solid black.
        #[rustfmt::skip]
        let src = vec![
            0, 255, 0, 255,
            0, 255, 0, 255,
            0, 255, 0, 255,
            0, 255, 0, 255,
        ];
        let out = resize_grayscale(&src, 4, 4, 2, 2);
        assert_eq!(out.len(), 4);
        for &v in &out {
            assert!(
                (v as i32 - 128).abs() <= 5,
                "downscale must average 0/255 columns to ~128, got {v} in {out:?}"
            );
        }
    }

    /// Fill a `w`x`h` white canvas with a period-8 vertical-bar pattern of
    /// size `cw`x`ch` placed at (off_x, off_y).
    fn bars_on_canvas(w: u32, h: u32, cw: u32, ch: u32, off_x: u32, off_y: u32) -> Vec<u8> {
        let mut img = vec![255u8; (w * h) as usize];
        for y in 0..ch {
            for x in 0..cw {
                if (x % 8) < 4 {
                    let px = off_x + x;
                    let py = off_y + y;
                    img[(py * w + px) as usize] = 0;
                }
            }
        }
        img
    }

    #[test]
    fn test_registration_corrects_translation() {
        // Two rasters with IDENTICAL content but placed at different offsets
        // inside a white canvas (mmdc slack + sub-pixel translation). Plain
        // windowed SSIM compares them misaligned and collapses; registering to
        // the content bounding box must recover a near-1.0 score.
        let w = 100u32;
        let h = 100u32;
        let a = bars_on_canvas(w, h, 80, 80, 0, 0);
        let b = bars_on_canvas(w, h, 80, 80, 18, 18);

        let plain = calculate_ssim(&a, &b, w, h);
        let registered = calculate_ssim_registered(&a, w, h, &b, w, h);

        assert!(
            plain < 0.6,
            "misaligned identical content should score low without registration, got {plain}"
        );
        assert!(
            registered > 0.95,
            "registration should align identical content to near-1.0, got {registered}"
        );
    }

    #[test]
    fn test_registration_respects_aspect_ratio_band() {
        // Genuinely different layouts (wide vs tall content) must NOT be forced
        // into alignment: registration falls back to the plain resize compare
        // so a real divergence is not masked.
        let w = 100u32;
        let h = 100u32;
        // Wide content block (80x20) vs tall content block (20x80).
        let mut wide = vec![255u8; (w * h) as usize];
        for y in 10..30 {
            for x in 10..90 {
                wide[(y * w + x) as usize] = 0;
            }
        }
        let mut tall = vec![255u8; (w * h) as usize];
        for y in 10..90 {
            for x in 10..30 {
                tall[(y * w + x) as usize] = 0;
            }
        }

        let registered = calculate_ssim_registered(&wide, w, h, &tall, w, h);
        let fallback = calculate_ssim_with_resize(&wide, w, h, &tall, w, h);
        assert!(
            (registered - fallback).abs() < 1e-9,
            "aspect-ratio mismatch must fall back to resize compare, got {registered} vs {fallback}"
        );
    }

    #[test]
    fn test_floor_dims_scales_sub_floor_up_preserving_aspect() {
        // Below the floor, scale the long edge up to the floor and the short
        // edge proportionally.
        let (w, h) = floor_dims(100, 200);
        assert_eq!(h, MIN_REGISTERED_LONG_EDGE);
        assert_eq!(w, MIN_REGISTERED_LONG_EDGE / 2);

        // At or above the floor, leave dimensions untouched.
        assert_eq!(
            floor_dims(MIN_REGISTERED_LONG_EDGE, 300),
            (MIN_REGISTERED_LONG_EDGE, 300)
        );
        assert_eq!(floor_dims(1200, 800), (1200, 800));

        // Degenerate zero input is passed through, not divided by zero.
        assert_eq!(floor_dims(0, 0), (0, 0));
    }

    #[test]
    fn test_registration_applies_minimum_resolution_floor() {
        // Windowed SSIM deflates catastrophically as the compared raster
        // shrinks: the fixed 8x8 window covers a large fraction of a tiny image,
        // so a sub-pixel-scale rendering difference decorrelates nearly every
        // window. Here two 40x40 rasters carry the SAME structure - a shared
        // border frame plus five internal horizontal rules - with the rules
        // offset by a single pixel (the kind of sub-pixel positioning drift
        // between two faithful renderers). At native resolution the fixed window
        // straddles the offset rules and the score collapses toward zero. The
        // minimum render-scale floor upsamples the registered crops to a genuine
        // resolution so interior windows dominate and the score reflects the
        // near-identical structure - without collapsing to 1.0, so a real
        // difference stays visible.
        let w = 40u32;
        let h = 40u32;
        let mut a = vec![255u8; (w * h) as usize];
        let mut b = vec![255u8; (w * h) as usize];
        // shared border so both content boxes fill the raster (same AR).
        for i in 0..w {
            a[i as usize] = 0;
            b[i as usize] = 0;
            a[((h - 1) * w + i) as usize] = 0;
            b[((h - 1) * w + i) as usize] = 0;
        }
        for j in 0..h {
            a[(j * w) as usize] = 0;
            b[(j * w) as usize] = 0;
            a[(j * w + w - 1) as usize] = 0;
            b[(j * w + w - 1) as usize] = 0;
        }
        // five internal rules, offset by 1px between A and B.
        for k in 1..6 {
            for x in 1..w - 1 {
                a[((k * 6) * w + x) as usize] = 0;
                b[((k * 6 + 1) * w + x) as usize] = 0;
            }
        }

        let native = calculate_ssim_with_resize(&a, w, h, &b, w, h);
        let floored = calculate_ssim_registered(&a, w, h, &b, w, h);
        assert!(
            native < 0.3,
            "tiny raster with a 1px sub-pixel offset must collapse without the \
             floor (baseline of the bug), got {native}"
        );
        assert!(
            floored > 0.5,
            "the render-scale floor must lift near-identical tiny structure well \
             above the deflated native score, got {floored}"
        );
        assert!(
            floored < 0.95,
            "the floor must NOT collapse a genuine 1px difference to identity, \
             got {floored}"
        );
    }

    #[test]
    fn test_registration_floor_preserves_identical_tiny_content() {
        // Identical tiny content must still score ~1.0 after the floor upsample:
        // the floor adds resolution, never spurious dissimilarity.
        let w = 30u32;
        let h = 45u32;
        let mut img = vec![255u8; (w * h) as usize];
        for y in 5..40 {
            for x in 5..25 {
                img[(y * w + x) as usize] = 0;
            }
        }
        let ssim = calculate_ssim_registered(&img, w, h, &img, w, h);
        assert!(
            ssim > 0.99,
            "identical tiny content must stay ~1.0 through the floor, got {ssim}"
        );
    }

    #[test]
    fn test_registration_floor_does_not_mask_geometric_divergence() {
        // The floor must recalibrate the measurement basis WITHOUT masking real
        // geometric differences. Two same-aspect-ratio images whose content
        // genuinely diverges (a filled block vs a hollow outline of the same
        // bounds) must still score low even after floor upsampling.
        let w = 80u32;
        let h = 80u32;
        let mut filled = vec![255u8; (w * h) as usize];
        let mut outline = vec![255u8; (w * h) as usize];
        for y in 10..70 {
            for x in 10..70 {
                filled[(y * w + x) as usize] = 0;
                // outline: only the border ring is black
                if x == 10 || x == 69 || y == 10 || y == 69 {
                    outline[(y * w + x) as usize] = 0;
                }
            }
        }

        let registered = calculate_ssim_registered(&filled, w, h, &outline, w, h);
        assert!(
            registered < 0.9,
            "floor upsampling must not mask a genuine geometric difference, got {registered}"
        );
    }

    #[test]
    fn test_resize_preserves_thin_lines_as_gray() {
        // A 1px black line in a white image must survive a downscale as a
        // gray line, not vanish entirely (point sampling loses it).
        let src_w = 30u32;
        let src_h = 30u32;
        let mut src = vec![255u8; (src_w * src_h) as usize];
        // Horizontal black line on row 10
        for x in 0..src_w {
            src[(10 * src_w + x) as usize] = 0;
        }
        // 30 -> 13 point sampling hits rows 0,2,4,6,9,11,... and skips row 10,
        // so nearest-neighbor loses the line entirely.
        let dst_w = 13u32;
        let dst_h = 13u32;
        let out = resize_grayscale(&src, src_w, src_h, dst_w, dst_h);
        let min = *out.iter().min().unwrap();
        assert!(
            min < 250,
            "thin line must remain visible after downscale, got min {min}"
        );
    }
}
