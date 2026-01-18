//! Content-addressable cache for reference SVGs.
//!
//! Caches mermaid.js rendered SVGs by diagram content hash to avoid
//! re-rendering the same diagram multiple times.
//!
//! Uses mermaid.ink API for reference rendering (no local browser required).

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

/// Cache for reference SVG and PNG outputs
pub struct ReferenceCache {
    /// Directory where SVG cache files are stored
    cache_dir: PathBuf,
    /// Directory where PNG cache files are stored
    png_cache_dir: PathBuf,
}

impl ReferenceCache {
    /// Create a new cache with specified directory
    pub fn new(cache_dir: impl AsRef<Path>) -> Self {
        let cache_path = cache_dir.as_ref().to_path_buf();
        let png_path = cache_path.parent().unwrap_or(&cache_path).join("pngs");
        Self {
            cache_dir: cache_path,
            png_cache_dir: png_path,
        }
    }

    /// Create a cache using default paths.
    ///
    /// Cache directory: Platform-specific cache location + selkie/references/
    /// - macOS: ~/Library/Caches/selkie/references/
    /// - Linux: ~/.cache/selkie/references/
    /// - Windows: %LOCALAPPDATA%/selkie/references/
    ///
    /// PNG cache: selkie/pngs/
    ///
    /// Use `selkie eval --cache-info` to see the actual cache location.
    pub fn with_defaults() -> Self {
        let base_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("selkie");

        Self {
            cache_dir: base_dir.join("references"),
            png_cache_dir: base_dir.join("pngs"),
        }
    }

    /// Get the cache directory path
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Ensure the cache directory exists
    pub fn ensure_dir(&self) -> std::io::Result<()> {
        fs::create_dir_all(&self.cache_dir)
    }

    /// Get the cache path for a diagram
    fn cache_path(&self, diagram: &str) -> PathBuf {
        let hash = hash_diagram(diagram);
        self.cache_dir.join(format!("{}.svg", hash))
    }

    /// Check if a diagram is cached
    pub fn is_cached(&self, diagram: &str) -> bool {
        self.cache_path(diagram).exists()
    }

    /// Get a cached SVG, or None if not cached
    pub fn get(&self, diagram: &str) -> Option<String> {
        let path = self.cache_path(diagram);
        fs::read_to_string(path).ok()
    }

    /// Store an SVG in the cache
    pub fn put(&self, diagram: &str, svg: &str) -> std::io::Result<()> {
        self.ensure_dir()?;
        let path = self.cache_path(diagram);
        fs::write(path, svg)
    }

    /// Get or render a reference SVG
    ///
    /// If cached, returns the cached version.
    /// Otherwise, renders with mermaid.ink and caches the result.
    pub fn get_or_render(&self, diagram: &str) -> Result<String, String> {
        // Check cache first
        if let Some(svg) = self.get(diagram) {
            return Ok(svg);
        }

        // Render with mermaid.ink API
        let svg = self.render_with_mermaid(diagram)?;

        // Cache the result (ignore errors, caching is optional)
        let _ = self.put(diagram, &svg);

        Ok(svg)
    }

    /// Render multiple diagrams at once, using cache where available.
    /// Uses mermaid.ink API for uncached diagrams.
    /// Returns a vector of Result<svg, error> in the same order as input.
    pub fn render_batch(&self, diagrams: &[&str]) -> Vec<Result<String, String>> {
        // Check which diagrams need rendering
        let uncached: Vec<(usize, &str)> = diagrams
            .iter()
            .enumerate()
            .filter(|(_, d)| !self.is_cached(d))
            .map(|(i, d)| (i, *d))
            .collect();

        // If all cached, just return cached results
        if uncached.is_empty() {
            return diagrams
                .iter()
                .map(|d| self.get(d).ok_or_else(|| "Cache miss".to_string()))
                .collect();
        }

        eprint!(
            "Rendering {} reference SVGs with mermaid.ink...",
            uncached.len()
        );

        // Render uncached diagrams using mermaid.ink API
        let batch_results: Vec<Result<String, String>> = uncached
            .iter()
            .map(|(_, diagram)| self.render_with_mermaid_ink(diagram))
            .collect();

        eprintln!(" done");

        // Cache successful results
        for ((_, diagram), result) in uncached.iter().zip(batch_results.iter()) {
            if let Ok(ref svg) = result {
                let _ = self.put(diagram, svg);
            }
        }

        // Build final results: batch results for uncached, cache for cached
        let mut batch_iter = batch_results.into_iter();
        let mut uncached_idx = 0;

        diagrams
            .iter()
            .enumerate()
            .map(|(i, d)| {
                if uncached_idx < uncached.len() && uncached[uncached_idx].0 == i {
                    uncached_idx += 1;
                    batch_iter.next().unwrap()
                } else {
                    self.get(d).ok_or_else(|| "Cache miss".to_string())
                }
            })
            .collect()
    }

    /// Render a diagram using mermaid.ink API
    fn render_with_mermaid_ink(&self, diagram: &str) -> Result<String, String> {
        // Base64 encode the diagram
        let encoded = BASE64.encode(diagram.as_bytes());

        // Build the URL
        let url = format!("https://mermaid.ink/svg/{}", encoded);

        // Make the HTTP request and read response body
        let svg = ureq::get(&url)
            .timeout(std::time::Duration::from_secs(30))
            .call()
            .map_err(|e| format!("mermaid.ink request failed: {}", e))?
            .into_string()
            .map_err(|e| format!("Failed to read mermaid.ink response: {}", e))?;

        if svg.trim().is_empty() {
            return Err("mermaid.ink returned empty response".to_string());
        }

        // Check if the response is actually an SVG
        if !svg.contains("<svg") {
            return Err(format!("mermaid.ink returned non-SVG response: {}", svg));
        }

        Ok(svg)
    }

    /// Render a diagram using mermaid.ink API (public interface)
    pub fn render_with_mermaid(&self, diagram: &str) -> Result<String, String> {
        self.render_with_mermaid_ink(diagram)
    }

    // ==================== PNG Cache Methods ====================

    /// Get the PNG cache directory path
    pub fn png_cache_dir(&self) -> &Path {
        &self.png_cache_dir
    }

    /// Ensure the PNG cache directory exists
    fn ensure_png_dir(&self) -> std::io::Result<()> {
        fs::create_dir_all(&self.png_cache_dir)
    }

    /// Get the PNG cache path for a diagram
    fn png_cache_path(&self, diagram: &str) -> PathBuf {
        let hash = hash_diagram(diagram);
        self.png_cache_dir.join(format!("{}.png", hash))
    }

    /// Check if a PNG is cached for this diagram
    pub fn is_png_cached(&self, diagram: &str) -> bool {
        self.png_cache_path(diagram).exists()
    }

    /// Get cached PNG data, or None if not cached
    pub fn get_png(&self, diagram: &str) -> Option<Vec<u8>> {
        let path = self.png_cache_path(diagram);
        fs::read(path).ok()
    }

    /// Store PNG data in the cache
    pub fn put_png(&self, diagram: &str, png_data: &[u8]) -> std::io::Result<()> {
        self.ensure_png_dir()?;
        let path = self.png_cache_path(diagram);
        fs::write(path, png_data)
    }

    /// Clear all cached files (SVG and PNG)
    pub fn clear(&self) -> std::io::Result<()> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)?;
        }
        if self.png_cache_dir.exists() {
            fs::remove_dir_all(&self.png_cache_dir)?;
        }
        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let mut stats = CacheStats::default();

        if let Ok(entries) = fs::read_dir(&self.cache_dir) {
            for entry in entries.flatten() {
                if entry
                    .path()
                    .extension()
                    .map(|e| e == "svg")
                    .unwrap_or(false)
                {
                    stats.count += 1;
                    if let Ok(metadata) = entry.metadata() {
                        stats.total_size += metadata.len();
                    }
                }
            }
        }

        stats
    }
}

/// Cache statistics
#[derive(Debug, Default)]
pub struct CacheStats {
    /// Number of cached files
    pub count: usize,
    /// Total size in bytes
    pub total_size: u64,
}

/// Generate a hash for a diagram source
fn hash_diagram(diagram: &str) -> String {
    let mut hasher = DefaultHasher::new();
    // Normalize whitespace for consistent hashing
    let normalized = diagram
        .lines()
        .map(|line| line.trim())
        .collect::<Vec<_>>()
        .join("\n");
    normalized.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn test_hash_consistency() {
        let diagram = "flowchart LR\n    A --> B";
        let hash1 = hash_diagram(diagram);
        let hash2 = hash_diagram(diagram);
        assert_eq!(hash1, hash2, "Same diagram should produce same hash");
    }

    #[test]
    fn test_hash_whitespace_normalization() {
        let diagram1 = "flowchart LR\n    A --> B";
        let diagram2 = "flowchart LR\n        A --> B";
        let hash1 = hash_diagram(diagram1);
        let hash2 = hash_diagram(diagram2);
        assert_eq!(hash1, hash2, "Whitespace differences should be normalized");
    }

    #[test]
    fn test_cache_operations() {
        let cache_dir = temp_dir().join("selkie_test_cache");
        let _ = fs::remove_dir_all(&cache_dir);

        let cache = ReferenceCache::new(&cache_dir);

        let diagram = "flowchart LR\n    A --> B";
        let svg = "<svg>test</svg>";

        // Initially not cached
        assert!(!cache.is_cached(diagram));

        // Store and retrieve
        cache.put(diagram, svg).unwrap();
        assert!(cache.is_cached(diagram));
        assert_eq!(cache.get(diagram), Some(svg.to_string()));

        // Clean up
        let _ = fs::remove_dir_all(&cache_dir);
    }
}
