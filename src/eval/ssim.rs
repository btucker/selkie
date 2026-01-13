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

/// Calculate SSIM between two images represented as grayscale pixel arrays
///
/// Both images must have the same dimensions.
/// Returns a value between 0 and 1 (1 = identical).
pub fn calculate_ssim(img1: &[u8], img2: &[u8], _width: u32, _height: u32) -> f64 {
    if img1.len() != img2.len() {
        return 0.0;
    }

    let n = img1.len() as f64;
    if n == 0.0 {
        return 1.0;
    }

    // Calculate means
    let mean1 = img1.iter().map(|&x| x as f64).sum::<f64>() / n;
    let mean2 = img2.iter().map(|&x| x as f64).sum::<f64>() / n;

    // Calculate variances and covariance
    let mut var1 = 0.0;
    let mut var2 = 0.0;
    let mut covar = 0.0;

    for (&p1, &p2) in img1.iter().zip(img2.iter()) {
        let d1 = p1 as f64 - mean1;
        let d2 = p2 as f64 - mean2;
        var1 += d1 * d1;
        var2 += d2 * d2;
        covar += d1 * d2;
    }

    var1 /= n - 1.0;
    var2 /= n - 1.0;
    covar /= n - 1.0;

    // SSIM constants
    let c1 = (K1 * L).powi(2);
    let c2 = (K2 * L).powi(2);

    // SSIM formula
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

/// Resize image to target dimensions using simple nearest-neighbor
///
/// This is a basic implementation for normalizing image sizes before comparison.
pub fn resize_grayscale(
    src: &[u8],
    src_width: u32,
    src_height: u32,
    dst_width: u32,
    dst_height: u32,
) -> Vec<u8> {
    let mut dst = vec![0u8; (dst_width * dst_height) as usize];

    for y in 0..dst_height {
        for x in 0..dst_width {
            let src_x = (x as f64 * src_width as f64 / dst_width as f64) as u32;
            let src_y = (y as f64 * src_height as f64 / dst_height as f64) as u32;
            let src_idx = (src_y * src_width + src_x) as usize;
            let dst_idx = (y * dst_width + x) as usize;

            if src_idx < src.len() {
                dst[dst_idx] = src[src_idx];
            }
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
}
