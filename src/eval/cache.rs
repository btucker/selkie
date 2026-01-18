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

/// Cache for reference SVG and PNG outputs
pub struct ReferenceCache {
    /// Directory where SVG cache files are stored (hash-based)
    cache_dir: PathBuf,
    /// Directory where PNG cache files are stored
    png_cache_dir: PathBuf,
    /// Optional repo-based cache directory (name-based, e.g., docs/images/reference/)
    repo_cache_dir: Option<PathBuf>,
}

impl ReferenceCache {
    /// Create a new cache with specified directory
    pub fn new(cache_dir: impl AsRef<Path>) -> Self {
        let cache_path = cache_dir.as_ref().to_path_buf();
        let png_path = cache_path.parent().unwrap_or(&cache_path).join("pngs");
        Self {
            cache_dir: cache_path,
            png_cache_dir: png_path,
            repo_cache_dir: None,
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
    /// Repo cache: docs/images/reference/ (if it exists)
    ///
    /// Use `selkie eval --cache-info` to see the actual cache location.
    pub fn with_defaults() -> Self {
        let base_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("selkie");

        // Check for repo-based cache directory
        let repo_cache = Path::new("docs/images/reference");
        let repo_cache_dir = if repo_cache.exists() {
            Some(repo_cache.to_path_buf())
        } else {
            None
        };

        Self {
            cache_dir: base_dir.join("references"),
            png_cache_dir: base_dir.join("pngs"),
            repo_cache_dir,
        }
    }

    /// Set or update the repo cache directory
    pub fn with_repo_cache(mut self, repo_cache_dir: impl AsRef<Path>) -> Self {
        self.repo_cache_dir = Some(repo_cache_dir.as_ref().to_path_buf());
        self
    }

    /// Get the repo cache directory path (if set)
    pub fn repo_cache_dir(&self) -> Option<&Path> {
        self.repo_cache_dir.as_deref()
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

    // ==================== Repo Cache Methods (Name-Based) ====================

    /// Ensure the repo cache directory exists
    fn ensure_repo_dir(&self) -> std::io::Result<()> {
        if let Some(ref dir) = self.repo_cache_dir {
            fs::create_dir_all(dir)
        } else {
            Ok(())
        }
    }

    /// Get the repo cache path for a named diagram
    fn repo_cache_path(&self, name: &str) -> Option<PathBuf> {
        self.repo_cache_dir
            .as_ref()
            .map(|dir| dir.join(format!("{}.svg", name)))
    }

    /// Check if a named diagram is in the repo cache and up-to-date
    ///
    /// Returns true only if the cached SVG exists AND was generated from
    /// the same diagram content (checked via embedded hash).
    pub fn is_cached_by_name(&self, name: &str, diagram: &str) -> bool {
        self.repo_cache_path(name)
            .and_then(|path| fs::read_to_string(path).ok())
            .map(|svg| self.check_embedded_hash(&svg, diagram))
            .unwrap_or(false)
    }

    /// Get a cached SVG by name from the repo cache, only if it matches the diagram content
    pub fn get_by_name(&self, name: &str, diagram: &str) -> Option<String> {
        self.repo_cache_path(name)
            .and_then(|path| fs::read_to_string(path).ok())
            .filter(|svg| self.check_embedded_hash(svg, diagram))
    }

    /// Store an SVG in the repo cache by name with embedded source hash
    pub fn put_by_name(&self, name: &str, diagram: &str, svg: &str) -> std::io::Result<()> {
        self.ensure_repo_dir()?;
        if let Some(path) = self.repo_cache_path(name) {
            let svg_with_hash = self.embed_source_hash(svg, diagram);
            fs::write(path, svg_with_hash)
        } else {
            Ok(()) // No repo cache configured, silently skip
        }
    }

    /// Embed source hash as an HTML comment at the end of the SVG
    fn embed_source_hash(&self, svg: &str, diagram: &str) -> String {
        let hash = hash_diagram(diagram);
        // Remove any existing hash comment first
        let svg_clean = self.strip_embedded_hash(svg);
        format!(
            "{}<!-- selkie-source-hash: {} -->\n",
            svg_clean.trim_end(),
            hash
        )
    }

    /// Check if the embedded hash matches the diagram content
    fn check_embedded_hash(&self, svg: &str, diagram: &str) -> bool {
        let expected_hash = hash_diagram(diagram);
        if let Some(embedded) = self.extract_embedded_hash(svg) {
            embedded == expected_hash
        } else {
            false // No hash found, treat as stale
        }
    }

    /// Extract the embedded source hash from an SVG
    fn extract_embedded_hash(&self, svg: &str) -> Option<String> {
        // Look for: <!-- selkie-source-hash: {hash} -->
        // The comment may be at the end of an existing line or on its own line
        let marker_start = "<!-- selkie-source-hash:";
        let marker_end = "-->";

        if let Some(start_pos) = svg.find(marker_start) {
            let after_marker = &svg[start_pos + marker_start.len()..];
            if let Some(end_pos) = after_marker.find(marker_end) {
                let hash = after_marker[..end_pos].trim();
                return Some(hash.to_string());
            }
        }
        None
    }

    /// Strip the embedded hash comment from an SVG
    fn strip_embedded_hash(&self, svg: &str) -> String {
        let marker_start = "<!-- selkie-source-hash:";
        let marker_end = "-->";

        if let Some(start_pos) = svg.find(marker_start) {
            if let Some(rel_end_pos) = svg[start_pos..].find(marker_end) {
                let end_pos = start_pos + rel_end_pos + marker_end.len();
                // Remove the marker and any trailing newline
                let before = &svg[..start_pos];
                let after = svg[end_pos..].trim_start_matches('\n');
                return format!("{}{}", before, after);
            }
        }
        svg.to_string()
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

    /// Get or render a reference SVG with name-based caching
    ///
    /// Cache lookup order:
    /// 1. Repo cache (docs/images/reference/{name}.svg) - by name, verified by content hash
    /// 2. Hash cache (~/.cache/selkie/references/{hash}.svg) - by content
    ///
    /// On render, saves to both caches.
    pub fn get_or_render_named(&self, name: &str, diagram: &str) -> Result<String, String> {
        // Check repo cache first (by name, with hash verification)
        if let Some(svg) = self.get_by_name(name, diagram) {
            return Ok(svg);
        }

        // Check hash cache
        if let Some(svg) = self.get(diagram) {
            // Save to repo cache for future lookups
            let _ = self.put_by_name(name, diagram, &svg);
            return Ok(svg);
        }

        // Render with mermaid.js
        let svg = self.render_with_mermaid(diagram)?;

        // Cache the result to both caches
        let _ = self.put_by_name(name, diagram, &svg);
        let _ = self.put(diagram, &svg);

        Ok(svg)
    }

    /// Render multiple diagrams at once, using cache where available.
    /// Uses mmdc batch mode (markdown input) for efficiency.
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

        eprint!("Rendering {} reference SVGs with mmdc...", uncached.len());

        // Render uncached diagrams in batch using markdown mode
        let batch_results = self.render_batch_with_mmdc(&uncached);

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

    /// Render multiple named diagrams at once, using repo cache (by name) and hash cache.
    /// Uses mmdc batch mode (markdown input) for efficiency.
    /// Returns a vector of Result<svg, error> in the same order as input.
    ///
    /// Cache lookup order:
    /// 1. Repo cache (docs/images/reference/{name}.svg) - by name, verified by content hash
    /// 2. Hash cache (~/.cache/selkie/references/{hash}.svg) - by content
    ///
    /// On render, saves to both caches. Automatically regenerates if source changes.
    pub fn render_batch_named(
        &self,
        named_diagrams: &[(&str, &str)],
    ) -> Vec<Result<String, String>> {
        // Check which diagrams need rendering (check repo cache first with hash verification, then hash cache)
        let uncached: Vec<(usize, &str, &str)> = named_diagrams
            .iter()
            .enumerate()
            .filter(|(_, (name, diagram))| {
                // Check repo cache first (by name, with hash verification)
                if self.is_cached_by_name(name, diagram) {
                    return false;
                }
                // Fall back to hash cache
                !self.is_cached(diagram)
            })
            .map(|(i, (name, diagram))| (i, *name, *diagram))
            .collect();

        // If all cached, just return cached results
        if uncached.is_empty() {
            return named_diagrams
                .iter()
                .map(|(name, diagram)| {
                    // Try repo cache first (with hash verification), then hash cache
                    self.get_by_name(name, diagram)
                        .or_else(|| self.get(diagram))
                        .ok_or_else(|| "Cache miss".to_string())
                })
                .collect();
        }

        eprint!("Rendering {} reference SVGs with mmdc...", uncached.len());

        // Render uncached diagrams in batch using markdown mode
        let diagrams_for_mmdc: Vec<(usize, &str)> =
            uncached.iter().map(|(i, _, d)| (*i, *d)).collect();
        let batch_results = self.render_batch_with_mmdc(&diagrams_for_mmdc);

        eprintln!(" done");

        // Cache successful results to BOTH caches
        for ((_, name, diagram), result) in uncached.iter().zip(batch_results.iter()) {
            if let Ok(ref svg) = result {
                // Save to repo cache (by name with embedded hash) - this is the primary cache
                let _ = self.put_by_name(name, diagram, svg);
                // Also save to hash cache for backwards compatibility
                let _ = self.put(diagram, svg);
            }
        }

        // Build final results: batch results for uncached, cache for cached
        let mut batch_iter = batch_results.into_iter();
        let mut uncached_idx = 0;

        named_diagrams
            .iter()
            .enumerate()
            .map(|(i, (name, diagram))| {
                if uncached_idx < uncached.len() && uncached[uncached_idx].0 == i {
                    uncached_idx += 1;
                    batch_iter.next().unwrap()
                } else {
                    // Try repo cache first (with hash verification), then hash cache
                    self.get_by_name(name, diagram)
                        .or_else(|| self.get(diagram))
                        .ok_or_else(|| "Cache miss".to_string())
                }
            })
            .collect()
    }

    /// Render multiple diagrams in a single mmdc invocation using markdown batch mode
    fn render_batch_with_mmdc(&self, diagrams: &[(usize, &str)]) -> Vec<Result<String, String>> {
        // Create temp directory for mmdc output
        let temp_dir = match tempfile::tempdir() {
            Ok(dir) => dir,
            Err(e) => {
                return diagrams
                    .iter()
                    .map(|_| Err(format!("Failed to create temp dir: {}", e)))
                    .collect();
            }
        };

        // Create markdown file with all diagrams as fenced code blocks
        let md_path = temp_dir.path().join("batch.md");
        let mut md_content = String::new();
        for (i, (_, diagram)) in diagrams.iter().enumerate() {
            md_content.push_str(&format!(
                "## Diagram {}\n\n```mermaid\n{}\n```\n\n",
                i, diagram
            ));
        }

        if let Err(e) = fs::write(&md_path, &md_content) {
            return diagrams
                .iter()
                .map(|_| Err(format!("Failed to write markdown: {}", e)))
                .collect();
        }

        // Run mmdc with markdown input
        let output_md = temp_dir.path().join("output.md");
        let artefacts_dir = temp_dir.path().join("svgs");

        let output = Command::new("mmdc")
            .args([
                "-i",
                md_path.to_str().unwrap(),
                "-o",
                output_md.to_str().unwrap(),
                "-a",
                artefacts_dir.to_str().unwrap(),
                "-q",
            ])
            .stderr(Stdio::piped())
            .output();

        let output = match output {
            Ok(o) => o,
            Err(e) => {
                return diagrams
                    .iter()
                    .map(|_| Err(format!("Failed to run mmdc: {}", e)))
                    .collect();
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return diagrams
                .iter()
                .map(|_| Err(format!("mmdc failed: {}", stderr)))
                .collect();
        }

        // Read generated SVGs (mmdc names them output-1.svg, output-2.svg, etc.)
        diagrams
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let svg_path = artefacts_dir.join(format!("output-{}.svg", i + 1));
                fs::read_to_string(&svg_path)
                    .map_err(|e| format!("Failed to read SVG {}: {}", svg_path.display(), e))
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

    #[test]
    fn test_embedded_hash() {
        let cache = ReferenceCache::with_defaults();
        let diagram = "flowchart LR\n    A --> B";
        let svg = "<svg>test</svg>";

        // Embed hash
        let svg_with_hash = cache.embed_source_hash(svg, diagram);
        assert!(svg_with_hash.contains("<!-- selkie-source-hash:"));
        assert!(svg_with_hash.ends_with(" -->\n"));

        // Extract hash
        let extracted = cache.extract_embedded_hash(&svg_with_hash);
        assert!(extracted.is_some());
        assert_eq!(extracted.unwrap(), hash_diagram(diagram));

        // Check hash validation
        assert!(cache.check_embedded_hash(&svg_with_hash, diagram));

        // Different diagram should not match
        let different_diagram = "flowchart LR\n    C --> D";
        assert!(!cache.check_embedded_hash(&svg_with_hash, different_diagram));
    }

    #[test]
    fn test_strip_embedded_hash() {
        let cache = ReferenceCache::with_defaults();
        let svg = "<svg>test</svg>\n<!-- selkie-source-hash: abc123 -->\n";

        let stripped = cache.strip_embedded_hash(svg);
        assert!(!stripped.contains("selkie-source-hash"));
        assert!(stripped.contains("<svg>test</svg>"));
    }

    #[test]
    fn test_repo_cache_operations() {
        let cache_dir = temp_dir().join("selkie_test_hash_cache");
        let repo_cache_dir = temp_dir().join("selkie_test_repo_cache");
        let _ = fs::remove_dir_all(&cache_dir);
        let _ = fs::remove_dir_all(&repo_cache_dir);

        let cache = ReferenceCache::new(&cache_dir).with_repo_cache(&repo_cache_dir);

        let name = "test_diagram";
        let diagram = "flowchart LR\n    A --> B";
        let svg = "<svg>test</svg>";

        // Initially not cached
        assert!(!cache.is_cached_by_name(name, diagram));

        // Store and retrieve with hash
        cache.put_by_name(name, diagram, svg).unwrap();
        assert!(cache.is_cached_by_name(name, diagram));
        let retrieved = cache.get_by_name(name, diagram);
        assert!(retrieved.is_some());
        assert!(retrieved.unwrap().contains("<svg>test</svg>"));

        // Different diagram content should not match
        let different_diagram = "flowchart LR\n    C --> D";
        assert!(!cache.is_cached_by_name(name, different_diagram));
        assert!(cache.get_by_name(name, different_diagram).is_none());

        // Clean up
        let _ = fs::remove_dir_all(&cache_dir);
        let _ = fs::remove_dir_all(&repo_cache_dir);
    }
}
