//! Content-addressable cache for reference SVGs.
//!
//! Caches mermaid.js rendered SVGs by diagram content hash to avoid
//! re-rendering the same diagram multiple times.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Cache for reference SVG outputs
pub struct ReferenceCache {
    /// Directory where cache files are stored
    cache_dir: PathBuf,
}

impl ReferenceCache {
    /// Create a new cache with specified directory
    pub fn new(cache_dir: impl AsRef<Path>) -> Self {
        Self {
            cache_dir: cache_dir.as_ref().to_path_buf(),
        }
    }

    /// Create a cache using default paths.
    ///
    /// Cache directory: Platform-specific cache location + selkie/references/
    /// - macOS: ~/Library/Caches/selkie/references/
    /// - Linux: ~/.cache/selkie/references/
    /// - Windows: %LOCALAPPDATA%/selkie/references/
    ///
    /// Use `selkie eval --cache-info` to see the actual cache location.
    pub fn with_defaults() -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("selkie")
            .join("references");

        Self::new(cache_dir)
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
    /// Otherwise, renders with mermaid.js and caches the result.
    pub fn get_or_render(&self, diagram: &str) -> Result<String, String> {
        // Check cache first
        if let Some(svg) = self.get(diagram) {
            return Ok(svg);
        }

        // Render with mermaid.js
        let svg = self.render_with_mermaid(diagram)?;

        // Cache the result (ignore errors, caching is optional)
        let _ = self.put(diagram, &svg);

        Ok(svg)
    }

    /// Render multiple diagrams at once, using cache where available.
    /// Returns a vector of (diagram, Result<svg, error>) in the same order as input.
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

        // Log progress for uncached diagrams
        if !uncached.is_empty() {
            eprint!("Rendering {} reference SVGs with mmdc...", uncached.len());
        }

        // Render uncached diagrams
        let mut results: Vec<Option<Result<String, String>>> = vec![None; diagrams.len()];

        for (i, diagram) in &uncached {
            let result = self.render_with_mermaid(diagram);
            if let Ok(ref svg) = result {
                let _ = self.put(diagram, svg);
            }
            results[*i] = Some(result);
        }

        if !uncached.is_empty() {
            eprintln!(" done");
        }

        // Fill in cached results
        diagrams
            .iter()
            .enumerate()
            .map(|(i, d)| {
                if let Some(result) = results[i].take() {
                    result
                } else {
                    self.get(d).ok_or_else(|| "Cache miss".to_string())
                }
            })
            .collect()
    }

    /// Render a diagram using mmdc (mermaid CLI)
    pub fn render_with_mermaid(&self, diagram: &str) -> Result<String, String> {
        // Check if mmdc is available
        let mmdc_check = Command::new("which")
            .arg("mmdc")
            .output()
            .map_err(|e| format!("Failed to check for mmdc: {}", e))?;

        if !mmdc_check.status.success() {
            return Err(
                "mmdc (mermaid CLI) is not installed. Install it with: npm install -g @mermaid-js/mermaid-cli"
                    .to_string(),
            );
        }

        // Run mmdc with stdin input and stdout output
        let mut child = Command::new("mmdc")
            .args(["-i", "-", "-o", "-", "-q"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn mmdc: {}", e))?;

        // Write diagram to stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(diagram.as_bytes())
                .map_err(|e| format!("Failed to write to mmdc stdin: {}", e))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|e| format!("Failed to get mmdc output: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("mmdc render failed: {}", stderr));
        }

        let svg = String::from_utf8_lossy(&output.stdout).to_string();

        if svg.trim().is_empty() {
            return Err("mmdc returned empty output".to_string());
        }

        Ok(svg)
    }

    /// Clear all cached files
    pub fn clear(&self) -> std::io::Result<()> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)?;
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
