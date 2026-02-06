//! selkie - A fast mermaid diagram renderer
//!
//! CLI interface compatible with mermaid-cli (mmdc) for easy migration.
//!
//! Usage:
//!   selkie input.mmd -o output.svg             # render is the default
//!   selkie -i input.mmd -o output.svg          # -i flag also works
//!   selkie render input.mmd -o output.svg      # explicit render subcommand
//!   selkie eval                                # evaluate with gallery samples
//!   selkie eval -o ./reports                   # custom output directory

use std::fs;
use std::io::{self, Read, Write};
#[cfg(feature = "eval")]
use std::path::Path;
use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand, ValueEnum};
use serde::Deserialize;
#[cfg(feature = "eval")]
use uuid::Uuid;

#[cfg(feature = "eval")]
use selkie::eval::{self, runner::DiagramInput, samples};
#[cfg(feature = "eval")]
use selkie::render::ascii as ascii_render;
use selkie::render::{RenderConfig, Theme};
use selkie::{parse, render_with_config};

/// Configuration file format (compatible with mermaid-cli)
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfigFile {
    /// Theme name
    #[serde(default)]
    theme: Option<String>,
    /// Custom theme variables
    #[serde(default)]
    theme_variables: Option<ThemeVariables>,
    /// Background color
    #[serde(default)]
    background: Option<String>,
}

/// Theme variable overrides
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThemeVariables {
    primary_color: Option<String>,
    primary_text_color: Option<String>,
    primary_border_color: Option<String>,
    secondary_color: Option<String>,
    tertiary_color: Option<String>,
    line_color: Option<String>,
    background: Option<String>,
    font_family: Option<String>,
}

/// A fast mermaid diagram renderer
#[derive(Parser, Debug)]
#[command(name = "selkie")]
#[command(version, about = "A fast mermaid diagram renderer")]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    // Flattened render args for backwards compatibility
    // When no subcommand is given but -i is provided, run render
    #[command(flatten)]
    render: RenderArgs,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Render a mermaid diagram to SVG/PNG/PDF
    Render(RenderArgs),
    /// Evaluate selkie against mermaid.js reference
    #[cfg(feature = "eval")]
    Eval(EvalArgs),
}

/// Arguments for the render command
#[derive(Parser, Debug, Default)]
struct RenderArgs {
    /// Input file (.mmd, .md) or - for stdin
    #[arg(value_name = "INPUT")]
    input_positional: Option<String>,

    /// Input file (.mmd, .md) or - for stdin (alternative to positional)
    #[arg(short, long, value_name = "FILE")]
    input: Option<String>,

    /// Output file (.svg) or - for stdout
    #[arg(short, long)]
    output: Option<String>,

    /// Theme for diagram colors
    #[arg(short, long, value_enum, default_value = "default")]
    theme: ThemeArg,

    /// Background color (e.g., "white", "#f0f0f0", "transparent")
    #[arg(short, long)]
    background: Option<String>,

    /// Output format (defaults to extension or svg)
    #[arg(short = 'e', long)]
    output_format: Option<OutputFormat>,

    /// Diagram width in pixels (not yet implemented)
    #[arg(short, long)]
    width: Option<u32>,

    /// Diagram height in pixels (not yet implemented)
    #[arg(short = 'H', long)]
    height: Option<u32>,

    /// Configuration file (JSON)
    #[arg(short = 'c', long)]
    config_file: Option<PathBuf>,

    /// Suppress console output
    #[arg(short, long)]
    quiet: bool,

    /// Show verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Display diagram directly in terminal (requires kitty/ghostty)
    #[cfg(feature = "kitty")]
    #[arg(short = 'd', long)]
    display: bool,

    /// Force terminal display even if kitty support is not detected
    #[cfg(feature = "kitty")]
    #[arg(long)]
    force_display: bool,
}

/// Arguments for the eval command
#[cfg(feature = "eval")]
#[derive(Parser, Debug)]
#[command(after_help = "\
Examples:
  selkie eval                     Run with gallery samples (AI-agent friendly output)
  selkie eval -o ./reports        Output to custom directory
  selkie eval --type flowchart    Evaluate only flowchart samples
  selkie eval ./diagrams/         Evaluate .mmd files from directory
  selkie eval --brief             Compact summary output
  selkie eval --verbose           Show detailed per-diagram diffs
")]
struct EvalArgs {
    /// Input to evaluate: JSON file, directory, .mmd file, or omit for gallery samples
    #[arg(value_name = "TARGET")]
    target: Option<String>,

    /// Filter by diagram type (flowchart, sequence, pie, etc.)
    #[arg(short = 't', long = "type")]
    diagram_type: Option<String>,

    /// Output directory for report (default: ./eval-report). Creates selkie-eval-XXXX subdirectory.
    #[arg(short, long, value_name = "DIR")]
    output: Option<PathBuf>,

    /// Show detailed diff per diagram (legacy format)
    #[arg(short, long)]
    verbose: bool,

    /// Compact summary output (disables default AI-agent friendly format)
    #[arg(short, long)]
    brief: bool,

    /// Clear cache and re-render all reference SVGs
    #[arg(long)]
    force_refresh: bool,

    /// Show cache location and statistics, then exit
    #[arg(long)]
    cache_info: bool,

    /// Open HTML report in default browser after evaluation
    #[arg(long)]
    open_report: bool,

    /// Use pre-committed SVGs from docs/images/reference/ instead of rendering with mmdc.
    /// Useful in CI where Playwright/Chromium may not be available.
    #[arg(long)]
    use_repo_svgs: bool,

    /// Evaluate ASCII output instead of SVG (only flowchart diagrams)
    #[arg(long)]
    ascii: bool,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, ValueEnum)]
enum ThemeArg {
    #[default]
    Default,
    Dark,
    Forest,
    Neutral,
    /// Auto-detect based on terminal background color
    #[cfg(feature = "kitty")]
    Auto,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    Svg,
    #[cfg(feature = "png")]
    Png,
    #[cfg(feature = "pdf")]
    Pdf,
    /// Character-art ASCII output
    Ascii,
}

impl OutputFormat {
    /// Detect output format from file extension
    fn from_extension(path: &str) -> Option<Self> {
        let path_lower = path.to_lowercase();
        if path_lower.ends_with(".svg") {
            Some(OutputFormat::Svg)
        } else if path_lower.ends_with(".png") {
            #[cfg(feature = "png")]
            return Some(OutputFormat::Png);
            #[cfg(not(feature = "png"))]
            return None;
        } else if path_lower.ends_with(".pdf") {
            #[cfg(feature = "pdf")]
            return Some(OutputFormat::Pdf);
            #[cfg(not(feature = "pdf"))]
            return None;
        } else {
            None
        }
    }
}

fn main() {
    let args = Args::parse();

    if let Err(e) = run(args) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    match args.command {
        Some(Commands::Render(render_args)) => run_render(render_args),
        #[cfg(feature = "eval")]
        Some(Commands::Eval(eval_args)) => run_eval(eval_args),
        // Default to render when no subcommand is specified
        None => run_render(args.render),
    }
}

fn run_render(args: RenderArgs) -> Result<(), Box<dyn std::error::Error>> {
    // Positional input takes precedence over -i flag
    let input_path = args
        .input_positional
        .as_ref()
        .or(args.input.as_ref())
        .ok_or("Input file is required. Usage: selkie <INPUT> [-o OUTPUT]")?;

    // Read input
    let input = read_input(input_path)?;

    if args.verbose {
        eprintln!("Read {} bytes from input", input.len());
    }

    // Load config file if specified
    let config_file = if let Some(ref path) = args.config_file {
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read config file: {}", e))?;
        let cfg: ConfigFile = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse config file: {}", e))?;
        if args.verbose {
            eprintln!("Loaded config from {}", path.display());
        }
        Some(cfg)
    } else {
        None
    };

    // Build render config - CLI args override config file
    let mut theme = match args.theme {
        ThemeArg::Default => {
            // Check config file for theme
            if let Some(ref cfg) = config_file {
                match cfg.theme.as_deref() {
                    Some("dark") => Theme::dark(),
                    Some("forest") => Theme::forest(),
                    Some("neutral") => Theme::neutral(),
                    _ => Theme::default(),
                }
            } else {
                Theme::default()
            }
        }
        ThemeArg::Dark => Theme::dark(),
        ThemeArg::Forest => Theme::forest(),
        ThemeArg::Neutral => Theme::neutral(),
        #[cfg(feature = "kitty")]
        ThemeArg::Auto => {
            // Auto-detect based on terminal background
            if selkie::kitty::is_terminal_dark() {
                if args.verbose {
                    eprintln!("Auto-detected dark terminal, using dark theme");
                }
                Theme::dark()
            } else {
                if args.verbose {
                    eprintln!("Auto-detected light terminal, using default theme");
                }
                Theme::default()
            }
        }
    };

    // Apply theme variables from config file
    if let Some(ref cfg) = config_file {
        if let Some(ref vars) = cfg.theme_variables {
            if let Some(ref c) = vars.primary_color {
                theme.primary_color = c.clone();
            }
            if let Some(ref c) = vars.primary_text_color {
                theme.primary_text_color = c.clone();
            }
            if let Some(ref c) = vars.primary_border_color {
                theme.primary_border_color = c.clone();
            }
            if let Some(ref c) = vars.secondary_color {
                theme.secondary_color = c.clone();
            }
            if let Some(ref c) = vars.tertiary_color {
                theme.tertiary_color = c.clone();
            }
            if let Some(ref c) = vars.line_color {
                theme.line_color = c.clone();
            }
            if let Some(ref c) = vars.background {
                theme.background = c.clone();
            }
            if let Some(ref f) = vars.font_family {
                theme.font_family = f.clone();
            }
        }
        // Apply background from config file
        if let Some(ref bg) = cfg.background {
            if bg == "transparent" {
                theme.background = "none".to_string();
            } else {
                theme.background = bg.clone();
            }
        }
    }

    // CLI background flag overrides config file
    if let Some(ref bg) = args.background {
        if bg == "transparent" {
            theme.background = "none".to_string();
        } else {
            theme.background = bg.clone();
        }
    }

    let config = RenderConfig {
        theme,
        ..RenderConfig::default()
    };

    // Parse the diagram
    let diagram = parse(&input).map_err(|e| format!("Parse error: {}", e))?;

    if args.verbose {
        eprintln!("Parsed diagram successfully");
    }

    // Check if ASCII format is requested — uses a separate render path
    let format_hint = args.output_format.unwrap_or_else(|| {
        args.output
            .as_deref()
            .and_then(|p| {
                if p == "-" {
                    None
                } else {
                    OutputFormat::from_extension(p)
                }
            })
            .unwrap_or(OutputFormat::Svg)
    });

    if format_hint == OutputFormat::Ascii {
        let output_str = selkie::render_ascii(&diagram)?;
        if args.verbose {
            eprintln!("Rendered {} bytes of ASCII output", output_str.len());
        }
        write_output(&args.output, output_str.as_bytes())?;
        if !args.quiet && args.output.as_deref() != Some("-") {
            if let Some(ref output) = args.output {
                eprintln!("Created {}", output);
            }
        }
        return Ok(());
    }

    // Render to SVG
    let svg = render_with_config(&diagram, &config).map_err(|e| format!("Render error: {}", e))?;

    if args.verbose {
        eprintln!("Rendered {} bytes of SVG", svg.len());
    }

    // Handle terminal display mode
    #[cfg(feature = "kitty")]
    if args.display || args.force_display {
        // Check for kitty support
        if !args.force_display && !selkie::kitty::is_supported() {
            return Err("Terminal does not support kitty graphics protocol. Use --force-display to override.".into());
        }

        if args.verbose {
            eprintln!("Displaying diagram in terminal using kitty graphics protocol");
        }

        // Convert to PNG for display
        let png_data = svg_to_png(&svg, args.width, args.height)?;
        selkie::kitty::display_png(&png_data)
            .map_err(|e| format!("Failed to display image: {}", e))?;

        // Also write to file if output was specified
        if let Some(ref output) = args.output {
            if output != "-" {
                let format = args.output_format.unwrap_or_else(|| {
                    OutputFormat::from_extension(output).unwrap_or(OutputFormat::Svg)
                });
                match format {
                    OutputFormat::Svg => write_output(&Some(output.clone()), svg.as_bytes())?,
                    #[cfg(feature = "png")]
                    OutputFormat::Png => write_binary_output(&Some(output.clone()), &png_data)?,
                    #[cfg(feature = "pdf")]
                    OutputFormat::Pdf => {
                        let pdf_data = svg_to_pdf(&svg)?;
                        write_binary_output(&Some(output.clone()), &pdf_data)?;
                    }
                    OutputFormat::Ascii => unreachable!("ASCII format handled above"),
                }
                if !args.quiet {
                    eprintln!("Created {}", output);
                }
            }
        }

        return Ok(());
    }

    // Determine output format
    let format = args.output_format.unwrap_or_else(|| {
        args.output
            .as_deref()
            .and_then(|p| {
                if p == "-" {
                    None
                } else {
                    OutputFormat::from_extension(p)
                }
            })
            .unwrap_or(OutputFormat::Svg)
    });

    // Write output based on format
    match format {
        OutputFormat::Svg => {
            write_output(&args.output, svg.as_bytes())?;
        }
        #[cfg(feature = "png")]
        OutputFormat::Png => {
            let png_data = svg_to_png(&svg, args.width, args.height)?;
            write_binary_output(&args.output, &png_data)?;
        }
        #[cfg(feature = "pdf")]
        OutputFormat::Pdf => {
            let pdf_data = svg_to_pdf(&svg)?;
            write_binary_output(&args.output, &pdf_data)?;
        }
        OutputFormat::Ascii => unreachable!("ASCII format handled above"),
    }

    if !args.quiet && args.output.as_deref() != Some("-") {
        if let Some(ref output) = args.output {
            eprintln!("Created {}", output);
        }
    }

    Ok(())
}

#[cfg(feature = "eval")]
fn run_eval(args: EvalArgs) -> Result<(), Box<dyn std::error::Error>> {
    let cache = eval::cache::ReferenceCache::with_defaults();

    // Handle --force-refresh: clear cache before re-rendering
    if args.force_refresh {
        let cache_dir = cache.cache_dir();
        if cache_dir.exists() {
            let stats = cache.stats();
            cache.clear()?;
            eprintln!(
                "Cleared {} cached files ({:.2} KB)",
                stats.count,
                stats.total_size as f64 / 1024.0,
            );
        }
    }

    // Handle --cache-info: show cache info and exit
    if args.cache_info {
        let stats = cache.stats();
        println!("Reference SVG Cache");
        println!("===================");
        println!("Location: {}", cache.cache_dir().display());
        println!("Files:    {}", stats.count);
        println!("Size:     {:.2} KB", stats.total_size as f64 / 1024.0);
        if stats.count == 0 {
            println!();
            println!("Cache is empty. Run 'selkie eval' to populate.");
        }
        return Ok(());
    }

    // Handle --ascii: run ASCII-specific evaluation
    if args.ascii {
        return run_eval_ascii(args);
    }

    // Build evaluation config
    // Enable visual comparison when png feature is available
    #[cfg(feature = "png")]
    let skip_visual = false;
    #[cfg(not(feature = "png"))]
    let skip_visual = true;

    let eval_config = eval::runner::EvalConfig {
        diagram_type_filter: args.diagram_type.clone(),
        skip_visual: skip_visual || args.use_repo_svgs,
        use_repo_svgs: args.use_repo_svgs,
        ..Default::default()
    };
    let runner = eval::runner::EvalRunner::new(eval_config, cache);

    // Get diagrams to evaluate
    let inputs = match &args.target {
        None => {
            // Use docs/sources/*.mmd files + embedded samples
            eprintln!("Using gallery samples (docs/sources/ + embedded)...");
            samples::all_samples_owned()
                .into_iter()
                .map(DiagramInput::from)
                .collect()
        }
        Some(target) => {
            let path = PathBuf::from(target);
            if path.is_dir() {
                // Evaluate all .mmd files in directory
                load_directory(&path)?
            } else if target.ends_with(".json") {
                // Load from JSON file (extract_diagrams output)
                load_json_diagrams(&path)?
            } else {
                // Single .mmd file
                let content = fs::read_to_string(&path)
                    .map_err(|e| format!("Failed to read {}: {}", target, e))?;
                vec![DiagramInput {
                    name: path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "diagram".to_string()),
                    source: Some(target.clone()),
                    diagram_type: None,
                    text: content,
                }]
            }
        }
    };

    if inputs.is_empty() {
        return Err("No diagrams to evaluate".into());
    }

    eprintln!("Evaluating {} diagrams...", inputs.len());

    // Run evaluation
    let result = runner.evaluate(&inputs);

    // Create output directory with random ID
    let base_dir = args
        .output
        .unwrap_or_else(|| PathBuf::from("./eval-report"));
    let random_id = &Uuid::new_v4().to_string()[..8];
    let output_dir = base_dir.join(format!("selkie-eval-{}", random_id));

    fs::create_dir_all(&output_dir)?;

    // Write HTML report as index.html
    eprint!("Writing HTML report...");
    let html_path = output_dir.join("index.html");
    eval::report::write_html(&result, &html_path)?;
    eprintln!(" done");

    // Write SVGs to subdirectories organized by diagram type
    eprint!("Writing SVG files...");

    // Also write to docs/images directories if they exist
    let docs_images = Path::new("docs/images");
    let docs_images_ref = Path::new("docs/images/reference");
    let write_to_docs = docs_images.exists() && docs_images_ref.exists();

    for diagram in &result.diagrams {
        let type_dir = output_dir.join(&diagram.diagram_type);
        let safe_name = diagram.name.replace(['/', ' '], "_");

        // Only create directory if we have at least one SVG to write
        if diagram.selkie_svg.is_some() || diagram.reference_svg.is_some() {
            fs::create_dir_all(&type_dir)?;
        }

        // Write selkie SVG if available
        if let Some(ref svg) = diagram.selkie_svg {
            let path = type_dir.join(format!("{}_selkie.svg", safe_name));
            fs::write(&path, svg)?;

            // Also write to docs/images/ (without _selkie suffix)
            if write_to_docs {
                let docs_path = docs_images.join(format!("{}.svg", safe_name));
                fs::write(&docs_path, svg)?;
            }
        }

        // Write reference SVG if available
        if let Some(ref svg) = diagram.reference_svg {
            let path = type_dir.join(format!("{}_reference.svg", safe_name));
            fs::write(&path, svg)?;

            // Also write to docs/images/reference/ (without _reference suffix)
            if write_to_docs {
                let docs_path = docs_images_ref.join(format!("{}.svg", safe_name));
                fs::write(&docs_path, svg)?;
            }
        }
    }
    eprintln!(" done");

    // Write comparison PNGs if png feature is enabled (requires both SVGs)
    let svg_pairs = runner.take_svg_pairs();
    #[cfg(feature = "png")]
    if !svg_pairs.is_empty() {
        eprint!(
            "Generating comparison PNGs ({} diagrams)...",
            svg_pairs.len()
        );
        match eval::png::write_comparison_pngs(&output_dir, &svg_pairs, runner.cache()) {
            Ok(_) => {
                eprintln!(" done");
            }
            Err(e) => {
                eprintln!(" failed");
                eprintln!("Warning: Failed to generate comparison PNGs: {}", e);
            }
        }
    }
    #[cfg(not(feature = "png"))]
    let _ = svg_pairs; // Suppress unused warning

    // Write JSON reports split by diagram type (easier for AI agents to read specific types)
    eval::report::write_json_by_type(&result, &output_dir)?;

    // Output results to stderr (default=agent, --verbose, or --brief)
    if args.brief {
        // Compact summary (old default)
        eprintln!("{}", eval::report::text_summary(&result, Some(&output_dir)));
    } else if args.verbose {
        // Detailed diff per diagram (legacy format)
        eprintln!(
            "{}",
            eval::report::text_detailed(&result, Some(&output_dir))
        );
    } else {
        // Default: AI-agent friendly output
        eprintln!(
            "{}",
            eval::report::text_agent_friendly(&result, Some(&output_dir))
        );
    }

    // Print the output directory path
    let report_path = output_dir.join("index.html");
    eprintln!("Evaluation report written to: {}", report_path.display());

    // Open report in browser if requested
    if args.open_report {
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg(&report_path).spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open")
                .arg(&report_path)
                .spawn();
        }
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("cmd")
                .args(["/C", "start", "", &report_path.to_string_lossy()])
                .spawn();
        }
    }

    // Exit with error code if there are failures
    if result.issue_counts.errors > 0 {
        process::exit(1);
    }

    Ok(())
}

/// Run ASCII-specific evaluation: parse → layout → render ASCII → parse ASCII → check
#[cfg(feature = "eval")]
fn run_eval_ascii(args: EvalArgs) -> Result<(), Box<dyn std::error::Error>> {
    use selkie::eval::ascii_checks;
    use selkie::layout::CharacterSizeEstimator;

    // Get diagrams to evaluate (reuse same loading logic)
    let inputs: Vec<DiagramInput> = match &args.target {
        None => {
            eprintln!("Using gallery samples (docs/sources/ + embedded)...");
            samples::all_samples_owned()
                .into_iter()
                .map(DiagramInput::from)
                .collect()
        }
        Some(target) => {
            let path = PathBuf::from(target);
            if path.is_dir() {
                load_directory(&path)?
            } else {
                let content = fs::read_to_string(&path)
                    .map_err(|e| format!("Failed to read {}: {}", target, e))?;
                vec![DiagramInput {
                    name: path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "diagram".to_string()),
                    source: Some(target.clone()),
                    diagram_type: None,
                    text: content,
                }]
            }
        }
    };

    // ASCII-supported diagram types (all diagram types now have ASCII renderers)
    let ascii_supported_types = [
        "flowchart",
        "sequence",
        "state",
        "class",
        "er",
        "architecture",
        "requirement",
        "mindmap",
        "pie",
        "gantt",
        "journey",
        "timeline",
        "kanban",
        "packet",
        "xychart",
        "quadrant",
        "radar",
        "git",
        "sankey",
        "block",
        "c4",
        "treemap",
    ];

    // Filter to ASCII-supported types, or a specific type if requested
    let ascii_diagrams: Vec<_> = inputs
        .iter()
        .filter(|i| {
            if let Some(ref filter) = args.diagram_type {
                i.diagram_type.as_deref() == Some(filter.as_str())
                    || detect_diagram_type(&i.text) == Some(filter.as_str())
            } else {
                // Default to all ASCII-supported types
                if let Some(ref dt) = i.diagram_type {
                    ascii_supported_types.contains(&dt.as_str())
                } else {
                    let detected = detect_diagram_type(&i.text);
                    detected.is_some_and(|t| ascii_supported_types.contains(&t))
                }
            }
        })
        .collect();

    if ascii_diagrams.is_empty() {
        return Err("No ASCII-supported diagrams to evaluate".into());
    }

    eprintln!(
        "Evaluating {} diagrams in ASCII mode...",
        ascii_diagrams.len()
    );

    let estimator = CharacterSizeEstimator::default();
    let mut total_issues = 0;
    let mut total_errors = 0;
    let mut total_diagrams = 0;
    let mut total_similarity = 0.0;

    for (i, input) in ascii_diagrams.iter().enumerate() {
        eprint!(
            "\rEvaluating {}/{}: {}...",
            i + 1,
            ascii_diagrams.len(),
            input.name
        );

        total_diagrams += 1;

        // Parse
        let parsed = match selkie::parse(&input.text) {
            Ok(p) => p,
            Err(e) => {
                eprintln!(" PARSE ERROR: {}", e);
                total_errors += 1;
                continue;
            }
        };

        // Pie charts don't use LayoutGraph — handle with dedicated eval path
        if let selkie::diagrams::Diagram::Pie(ref db) = parsed {
            let ascii_output = match ascii_render::pie::render_pie_ascii(db) {
                Ok(output) => output,
                Err(e) => {
                    eprintln!(" RENDER ERROR: {}", e);
                    total_errors += 1;
                    continue;
                }
            };

            let issues = ascii_checks::check_ascii_pie_structure(&ascii_output, db);
            let similarity = ascii_checks::calculate_ascii_pie_similarity(&ascii_output, db);
            total_similarity += similarity;

            let error_count = issues
                .iter()
                .filter(|i| i.level == eval::Level::Error)
                .count();
            let warning_count = issues
                .iter()
                .filter(|i| i.level == eval::Level::Warning)
                .count();
            total_issues += issues.len();
            total_errors += error_count;

            if args.verbose && !issues.is_empty() {
                eprintln!();
                eprintln!(
                    "  {} ({} errors, {} warnings, similarity: {:.1}%):",
                    input.name,
                    error_count,
                    warning_count,
                    similarity * 100.0
                );
                for issue in &issues {
                    let level = match issue.level {
                        eval::Level::Error => "ERROR",
                        eval::Level::Warning => "WARN",
                        eval::Level::Info => "INFO",
                    };
                    eprintln!("    [{}] {}: {}", level, issue.check, issue.message);
                }
            }
            continue;
        }

        // Sequence diagrams use their own renderer and eval checks (no LayoutGraph)
        if let selkie::diagrams::Diagram::Sequence(db) = &parsed {
            let ascii_output = match ascii_render::render_sequence_ascii(db) {
                Ok(output) => output,
                Err(e) => {
                    eprintln!(" RENDER ERROR: {}", e);
                    total_errors += 1;
                    continue;
                }
            };

            let ascii_struct = ascii_checks::parse_ascii_sequence(&ascii_output);
            let issues = ascii_checks::check_ascii_sequence_structure(&ascii_struct, db);
            let similarity = ascii_checks::calculate_ascii_sequence_similarity(&ascii_struct, db);
            total_similarity += similarity;

            let error_count = issues
                .iter()
                .filter(|i| i.level == eval::Level::Error)
                .count();
            let warning_count = issues
                .iter()
                .filter(|i| i.level == eval::Level::Warning)
                .count();
            total_issues += issues.len();
            total_errors += error_count;

            if args.verbose && !issues.is_empty() {
                eprintln!();
                eprintln!(
                    "  {} ({} errors, {} warnings, similarity: {:.1}%):",
                    input.name,
                    error_count,
                    warning_count,
                    similarity * 100.0
                );
                for issue in &issues {
                    let level = match issue.level {
                        eval::Level::Error => "ERROR",
                        eval::Level::Warning => "WARN",
                        eval::Level::Info => "INFO",
                    };
                    eprintln!("    [{}] {}: {}", level, issue.check, issue.message);
                }
            }
            continue;
        }

        // Gantt charts don't use LayoutGraph — handle with dedicated eval path
        if let selkie::diagrams::Diagram::Gantt(ref db) = parsed {
            let mut db_clone = db.clone();
            let ascii_output = match ascii_render::gantt::render_gantt_ascii(&mut db_clone) {
                Ok(output) => output,
                Err(e) => {
                    eprintln!(" RENDER ERROR: {}", e);
                    total_errors += 1;
                    continue;
                }
            };

            let issues = ascii_checks::check_ascii_gantt_structure(&ascii_output, &mut db_clone);
            let similarity =
                ascii_checks::calculate_ascii_gantt_similarity(&ascii_output, &mut db_clone);
            total_similarity += similarity;

            let error_count = issues
                .iter()
                .filter(|i| i.level == eval::Level::Error)
                .count();
            let warning_count = issues
                .iter()
                .filter(|i| i.level == eval::Level::Warning)
                .count();
            total_issues += issues.len();
            total_errors += error_count;

            if args.verbose && !issues.is_empty() {
                eprintln!();
                eprintln!(
                    "  {} ({} errors, {} warnings, similarity: {:.1}%):",
                    input.name,
                    error_count,
                    warning_count,
                    similarity * 100.0
                );
                for issue in &issues {
                    let level = match issue.level {
                        eval::Level::Error => "ERROR",
                        eval::Level::Warning => "WARN",
                        eval::Level::Info => "INFO",
                    };
                    eprintln!("    [{}] {}: {}", level, issue.check, issue.message);
                }
            }
            continue;
        }

        // Mindmap doesn't use LayoutGraph — handle with dedicated eval path
        if let selkie::diagrams::Diagram::Mindmap(ref db) = parsed {
            let ascii_output = match ascii_render::mindmap::render_mindmap_ascii(db) {
                Ok(output) => output,
                Err(e) => {
                    eprintln!(" RENDER ERROR: {}", e);
                    total_errors += 1;
                    continue;
                }
            };

            let issues = ascii_checks::check_ascii_mindmap_structure(&ascii_output, db);
            let similarity = ascii_checks::calculate_ascii_mindmap_similarity(&ascii_output, db);
            total_similarity += similarity;

            let error_count = issues
                .iter()
                .filter(|i| i.level == eval::Level::Error)
                .count();
            let warning_count = issues
                .iter()
                .filter(|i| i.level == eval::Level::Warning)
                .count();
            total_issues += issues.len();
            total_errors += error_count;

            if args.verbose && !issues.is_empty() {
                eprintln!();
                eprintln!(
                    "  {} ({} errors, {} warnings, similarity: {:.1}%):",
                    input.name,
                    error_count,
                    warning_count,
                    similarity * 100.0
                );
                for issue in &issues {
                    let level = match issue.level {
                        eval::Level::Error => "ERROR",
                        eval::Level::Warning => "WARN",
                        eval::Level::Info => "INFO",
                    };
                    eprintln!("    [{}] {}: {}", level, issue.check, issue.message);
                }
            }
            continue;
        }

        // Non-graph diagram types: render and compute simple text-based similarity
        let simple_eval_result = match &parsed {
            selkie::diagrams::Diagram::Journey(db) => {
                let output = ascii_render::journey::render_journey_ascii(db).ok();
                output.map(|o| {
                    let issues = ascii_checks::check_ascii_text_output(&o, "journey");
                    let similarity = ascii_checks::calculate_ascii_text_similarity(&o);
                    (issues, similarity)
                })
            }
            selkie::diagrams::Diagram::Timeline(db) => {
                let output = ascii_render::timeline::render_timeline_ascii(db).ok();
                output.map(|o| {
                    let issues = ascii_checks::check_ascii_text_output(&o, "timeline");
                    let similarity = ascii_checks::calculate_ascii_text_similarity(&o);
                    (issues, similarity)
                })
            }
            selkie::diagrams::Diagram::Kanban(db) => {
                let output = ascii_render::kanban::render_kanban_ascii(db).ok();
                output.map(|o| {
                    let issues = ascii_checks::check_ascii_text_output(&o, "kanban");
                    let similarity = ascii_checks::calculate_ascii_text_similarity(&o);
                    (issues, similarity)
                })
            }
            selkie::diagrams::Diagram::Packet(db) => {
                let output = ascii_render::packet::render_packet_ascii(db).ok();
                output.map(|o| {
                    let issues = ascii_checks::check_ascii_text_output(&o, "packet");
                    let similarity = ascii_checks::calculate_ascii_text_similarity(&o);
                    (issues, similarity)
                })
            }
            selkie::diagrams::Diagram::XyChart(db) => {
                let output = ascii_render::xychart::render_xychart_ascii(db).ok();
                output.map(|o| {
                    let issues = ascii_checks::check_ascii_text_output(&o, "xychart");
                    let similarity = ascii_checks::calculate_ascii_text_similarity(&o);
                    (issues, similarity)
                })
            }
            selkie::diagrams::Diagram::Quadrant(db) => {
                let output = ascii_render::quadrant::render_quadrant_ascii(db).ok();
                output.map(|o| {
                    let issues = ascii_checks::check_ascii_text_output(&o, "quadrant");
                    let similarity = ascii_checks::calculate_ascii_text_similarity(&o);
                    (issues, similarity)
                })
            }
            selkie::diagrams::Diagram::Radar(db) => {
                let output = ascii_render::radar::render_radar_ascii(db).ok();
                output.map(|o| {
                    let issues = ascii_checks::check_ascii_text_output(&o, "radar");
                    let similarity = ascii_checks::calculate_ascii_text_similarity(&o);
                    (issues, similarity)
                })
            }
            selkie::diagrams::Diagram::Git(db) => {
                let output = ascii_render::gitgraph::render_gitgraph_ascii(db).ok();
                output.map(|o| {
                    let issues = ascii_checks::check_ascii_text_output(&o, "git");
                    let similarity = ascii_checks::calculate_ascii_text_similarity(&o);
                    (issues, similarity)
                })
            }
            selkie::diagrams::Diagram::Sankey(db) => {
                let output = ascii_render::sankey::render_sankey_ascii(db).ok();
                output.map(|o| {
                    let issues = ascii_checks::check_ascii_text_output(&o, "sankey");
                    let similarity = ascii_checks::calculate_ascii_text_similarity(&o);
                    (issues, similarity)
                })
            }
            selkie::diagrams::Diagram::Block(db) => {
                let output = ascii_render::block::render_block_ascii(db).ok();
                output.map(|o| {
                    let issues = ascii_checks::check_ascii_text_output(&o, "block");
                    let similarity = ascii_checks::calculate_ascii_text_similarity(&o);
                    (issues, similarity)
                })
            }
            selkie::diagrams::Diagram::C4(db) => {
                let output = ascii_render::c4::render_c4_ascii(db).ok();
                output.map(|o| {
                    let issues = ascii_checks::check_ascii_text_output(&o, "c4");
                    let similarity = ascii_checks::calculate_ascii_text_similarity(&o);
                    (issues, similarity)
                })
            }
            selkie::diagrams::Diagram::Treemap(db) => {
                let output = ascii_render::treemap::render_treemap_ascii(db).ok();
                output.map(|o| {
                    let issues = ascii_checks::check_ascii_text_output(&o, "treemap");
                    let similarity = ascii_checks::calculate_ascii_text_similarity(&o);
                    (issues, similarity)
                })
            }
            _ => None,
        };

        if let Some((issues, similarity)) = simple_eval_result {
            total_similarity += similarity;
            let error_count = issues
                .iter()
                .filter(|i| i.level == eval::Level::Error)
                .count();
            let warning_count = issues
                .iter()
                .filter(|i| i.level == eval::Level::Warning)
                .count();
            total_issues += issues.len();
            total_errors += error_count;

            if args.verbose && !issues.is_empty() {
                eprintln!();
                eprintln!(
                    "  {} ({} errors, {} warnings, similarity: {:.1}%):",
                    input.name,
                    error_count,
                    warning_count,
                    similarity * 100.0
                );
                for issue in &issues {
                    let level = match issue.level {
                        eval::Level::Error => "ERROR",
                        eval::Level::Warning => "WARN",
                        eval::Level::Info => "INFO",
                    };
                    eprintln!("    [{}] {}: {}", level, issue.check, issue.message);
                }
            }
            continue;
        }

        // All other diagram types use ToLayoutGraph + generic ASCII renderer
        let graph = match layout_diagram(&parsed, &estimator) {
            Ok(g) => match selkie::layout::layout(g) {
                Ok(g) => g,
                Err(e) => {
                    eprintln!(" LAYOUT ERROR: {}", e);
                    total_errors += 1;
                    continue;
                }
            },
            Err(e) => {
                eprintln!(" LAYOUT ERROR: {}", e);
                total_errors += 1;
                continue;
            }
        };

        let ascii_output = match selkie::render_ascii(&parsed) {
            Ok(output) => output,
            Err(e) => {
                eprintln!(" RENDER ERROR: {}", e);
                total_errors += 1;
                continue;
            }
        };

        let ascii_struct = ascii_checks::parse_ascii(&ascii_output);

        // Run checks (generic + diagram-type-specific)
        let mut issues = ascii_checks::check_ascii_structure(&ascii_struct, &graph);
        // ER-specific checks: verify attributes and table structure
        if let selkie::diagrams::Diagram::Er(ref db) = parsed {
            issues.extend(ascii_checks::check_er_ascii_structure(&ascii_struct, db));
        }
        // State-specific checks: verify fork/join bars and composite containers
        if matches!(parsed, selkie::diagrams::Diagram::State(_)) {
            issues.extend(ascii_checks::check_ascii_state_structure(
                &ascii_struct,
                &graph,
            ));
        }
        let similarity = ascii_checks::calculate_ascii_similarity(&ascii_struct, &graph);
        total_similarity += similarity;

        let error_count = issues
            .iter()
            .filter(|i| i.level == eval::Level::Error)
            .count();
        let warning_count = issues
            .iter()
            .filter(|i| i.level == eval::Level::Warning)
            .count();
        total_issues += issues.len();
        total_errors += error_count;

        if args.verbose && !issues.is_empty() {
            eprintln!();
            eprintln!(
                "  {} ({} errors, {} warnings, similarity: {:.1}%):",
                input.name,
                error_count,
                warning_count,
                similarity * 100.0
            );
            for issue in &issues {
                let level = match issue.level {
                    eval::Level::Error => "ERROR",
                    eval::Level::Warning => "WARN",
                    eval::Level::Info => "INFO",
                };
                eprintln!("    [{}] {}: {}", level, issue.check, issue.message);
            }
        }
    }
    eprintln!();

    // Summary
    let avg_similarity = if total_diagrams > 0 {
        total_similarity / total_diagrams as f64
    } else {
        0.0
    };

    eprintln!("ASCII Evaluation Summary");
    eprintln!("======================");
    eprintln!("Diagrams:   {}", total_diagrams);
    eprintln!("Issues:     {} ({} errors)", total_issues, total_errors);
    eprintln!("Similarity: {:.1}% avg", avg_similarity * 100.0);

    if total_errors > 0 {
        process::exit(1);
    }

    Ok(())
}

/// Load diagrams from a directory of .mmd files
#[cfg(feature = "eval")]
fn load_directory(dir: &Path) -> Result<Vec<DiagramInput>, Box<dyn std::error::Error>> {
    let pattern = dir.join("**/*.mmd").to_string_lossy().to_string();
    let mut inputs = Vec::new();

    for entry in glob::glob(&pattern)? {
        let path = entry?;
        let content = fs::read_to_string(&path)?;
        inputs.push(DiagramInput {
            name: path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "diagram".to_string()),
            source: Some(path.to_string_lossy().to_string()),
            diagram_type: None,
            text: content,
        });
    }

    Ok(inputs)
}

/// Load diagrams from JSON file (extract_diagrams output format)
#[cfg(feature = "eval")]
fn load_json_diagrams(path: &PathBuf) -> Result<Vec<DiagramInput>, Box<dyn std::error::Error>> {
    #[derive(Deserialize)]
    struct JsonDiagram {
        name: Option<String>,
        #[serde(alias = "type")]
        diagram_type: Option<String>,
        source: String,
    }

    let content = fs::read_to_string(path)?;
    let diagrams: Vec<JsonDiagram> = serde_json::from_str(&content)?;

    Ok(diagrams
        .into_iter()
        .enumerate()
        .map(|(i, d)| DiagramInput {
            name: d.name.unwrap_or_else(|| format!("diagram_{}", i)),
            source: Some(path.to_string_lossy().to_string()),
            diagram_type: d.diagram_type,
            text: d.source,
        })
        .collect())
}

fn read_input(input: &str) -> Result<String, Box<dyn std::error::Error>> {
    if input == "-" {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        Ok(buffer)
    } else {
        Ok(fs::read_to_string(input)?)
    }
}

fn write_output(output: &Option<String>, content: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    match output.as_deref() {
        Some("-") | None => {
            io::stdout().write_all(content)?;
        }
        Some(path) => {
            fs::write(path, content)?;
        }
    }
    Ok(())
}

#[cfg(any(feature = "png", feature = "pdf"))]
fn write_binary_output(
    output: &Option<String>,
    content: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    match output.as_deref() {
        Some("-") | None => {
            io::stdout().write_all(content)?;
        }
        Some(path) => {
            fs::write(path, content)?;
        }
    }
    Ok(())
}

/// Convert SVG string to PNG bytes using resvg
#[cfg(feature = "png")]
fn svg_to_png(
    svg: &str,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use resvg::tiny_skia;
    use resvg::usvg;

    // Set up options with font database
    let mut opt = usvg::Options::default();
    let fontdb = opt.fontdb_mut();
    fontdb.load_system_fonts();

    // Set default font families to use when specified fonts aren't found
    // This ensures text renders even if "trebuchet ms" isn't available
    fontdb.set_sans_serif_family("Arial");
    fontdb.set_serif_family("Times New Roman");
    fontdb.set_monospace_family("Courier New");

    // Parse SVG
    let tree =
        usvg::Tree::from_str(svg, &opt).map_err(|e| format!("Failed to parse SVG: {}", e))?;

    // Calculate dimensions
    let svg_size = tree.size();
    let (target_width, target_height) = match (width, height) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => {
            let scale = w as f32 / svg_size.width();
            (w, (svg_size.height() * scale) as u32)
        }
        (None, Some(h)) => {
            let scale = h as f32 / svg_size.height();
            ((svg_size.width() * scale) as u32, h)
        }
        (None, None) => (svg_size.width() as u32, svg_size.height() as u32),
    };

    // Create pixmap
    let mut pixmap =
        tiny_skia::Pixmap::new(target_width, target_height).ok_or("Failed to create pixmap")?;

    // Calculate transform to fit
    let scale_x = target_width as f32 / svg_size.width();
    let scale_y = target_height as f32 / svg_size.height();
    let transform = tiny_skia::Transform::from_scale(scale_x, scale_y);

    // Render
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Encode to PNG
    let png_data = pixmap
        .encode_png()
        .map_err(|e| format!("Failed to encode PNG: {}", e))?;

    Ok(png_data)
}

/// Convert SVG string to PDF bytes using svg2pdf
#[cfg(feature = "pdf")]
fn svg_to_pdf(svg: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use resvg::usvg;

    // Set up options with font database
    let mut opt = usvg::Options::default();
    let fontdb = opt.fontdb_mut();
    fontdb.load_system_fonts();

    // Set default font families to use when specified fonts aren't found
    // This ensures text renders even if "trebuchet ms" isn't available
    fontdb.set_sans_serif_family("Arial");
    fontdb.set_serif_family("Times New Roman");
    fontdb.set_monospace_family("Courier New");

    // Parse SVG
    let tree =
        usvg::Tree::from_str(svg, &opt).map_err(|e| format!("Failed to parse SVG: {}", e))?;

    // Convert to PDF
    let pdf_data = svg2pdf::to_pdf(
        &tree,
        svg2pdf::ConversionOptions::default(),
        svg2pdf::PageOptions::default(),
    )
    .map_err(|e| format!("Failed to convert to PDF: {}", e))?;

    Ok(pdf_data)
}

/// Detect diagram type from raw mermaid text.
#[cfg(feature = "eval")]
fn detect_diagram_type(text: &str) -> Option<&str> {
    let lower = text.trim().to_lowercase();
    if lower.starts_with("flowchart") || lower.starts_with("graph ") {
        Some("flowchart")
    } else if lower.starts_with("statediagram") {
        Some("state")
    } else if lower.starts_with("classdiagram") || lower.starts_with("class") {
        Some("class")
    } else if lower.starts_with("erdiagram") {
        Some("er")
    } else if lower.starts_with("architecture") {
        Some("architecture")
    } else if lower.starts_with("requirement") {
        Some("requirement")
    } else if lower.starts_with("sequencediagram") || lower.starts_with("sequence") {
        Some("sequence")
    } else if lower.starts_with("gantt") {
        Some("gantt")
    } else if lower.starts_with("mindmap") {
        Some("mindmap")
    } else if lower.starts_with("pie") {
        Some("pie")
    } else {
        None
    }
}

/// Get a LayoutGraph from any diagram type that implements ToLayoutGraph.
#[cfg(feature = "eval")]
fn layout_diagram(
    diagram: &selkie::diagrams::Diagram,
    estimator: &selkie::layout::CharacterSizeEstimator,
) -> selkie::error::Result<selkie::layout::LayoutGraph> {
    use selkie::layout::ToLayoutGraph;

    match diagram {
        selkie::diagrams::Diagram::Flowchart(db) => db.to_layout_graph(estimator),
        selkie::diagrams::Diagram::State(db) => db.to_layout_graph(estimator),
        selkie::diagrams::Diagram::Class(db) => db.to_layout_graph(estimator),
        selkie::diagrams::Diagram::Er(db) => db.to_layout_graph(estimator),
        selkie::diagrams::Diagram::Architecture(db) => db.to_layout_graph(estimator),
        selkie::diagrams::Diagram::Requirement(db) => db.to_layout_graph(estimator),
        _ => Err(selkie::error::MermaidError::RenderError(
            "Diagram type does not support layout graph".to_string(),
        )),
    }
}
