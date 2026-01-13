//! selkie - A fast mermaid diagram renderer
//!
//! CLI interface compatible with mermaid-cli (mmdc) for easy migration.
//!
//! Usage:
//!   selkie render -i input.mmd -o output.svg
//!   selkie -i input.mmd -o output.svg          # implicit render
//!   selkie eval                                 # evaluate with gallery samples
//!   selkie eval --type flowchart --html report.html

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process;

use clap::{Parser, Subcommand, ValueEnum};
use serde::Deserialize;

use mermaid::eval::{self, runner::DiagramInput, samples};
use mermaid::render::{RenderConfig, Theme};
use mermaid::{parse, render_with_config};

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
    Eval(EvalArgs),
}

/// Arguments for the render command
#[derive(Parser, Debug, Default)]
struct RenderArgs {
    /// Input file (.mmd, .md) or - for stdin
    #[arg(short, long)]
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
#[derive(Parser, Debug)]
struct EvalArgs {
    /// Input to evaluate: JSON file, directory, .mmd file, or omit for gallery samples
    #[arg(value_name = "TARGET")]
    target: Option<String>,

    /// Filter by diagram type (flowchart, sequence, pie, etc.)
    #[arg(short = 't', long = "type")]
    diagram_type: Option<String>,

    /// Write JSON report to file
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Generate visual comparison HTML
    #[arg(long)]
    html: Option<PathBuf>,

    /// Save comparison PNGs for AI review
    #[arg(long)]
    pngs: Option<PathBuf>,

    /// Show detailed diff per diagram
    #[arg(short, long)]
    verbose: bool,

    /// Re-render all reference SVGs (ignore cache)
    #[arg(long)]
    force_refresh: bool,
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
        Some(Commands::Eval(eval_args)) => run_eval(eval_args),
        None => {
            // Backwards compatibility: if -i is provided without subcommand, run render
            if args.render.input.is_some() {
                run_render(args.render)
            } else {
                // No input and no subcommand - show help
                eprintln!("Usage: selkie [render] -i <INPUT> -o <OUTPUT>");
                eprintln!("       selkie eval [OPTIONS] [TARGET]");
                eprintln!();
                eprintln!("Run 'selkie --help' for more information.");
                process::exit(1);
            }
        }
    }
}

fn run_render(args: RenderArgs) -> Result<(), Box<dyn std::error::Error>> {
    let input_path = args.input.as_ref().ok_or("Input file is required")?;

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
            if mermaid::kitty::is_terminal_dark() {
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

    // Render to SVG
    let svg = render_with_config(&diagram, &config).map_err(|e| format!("Render error: {}", e))?;

    if args.verbose {
        eprintln!("Rendered {} bytes of SVG", svg.len());
    }

    // Handle terminal display mode
    #[cfg(feature = "kitty")]
    if args.display || args.force_display {
        // Check for kitty support
        if !args.force_display && !mermaid::kitty::is_supported() {
            return Err("Terminal does not support kitty graphics protocol. Use --force-display to override.".into());
        }

        if args.verbose {
            eprintln!("Displaying diagram in terminal using kitty graphics protocol");
        }

        // Convert to PNG for display
        let png_data = svg_to_png(&svg, args.width, args.height)?;
        mermaid::kitty::display_png(&png_data)
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
    }

    if !args.quiet && args.output.as_deref() != Some("-") {
        if let Some(ref output) = args.output {
            eprintln!("Created {}", output);
        }
    }

    Ok(())
}

fn run_eval(args: EvalArgs) -> Result<(), Box<dyn std::error::Error>> {
    // Build evaluation config
    let eval_config = eval::runner::EvalConfig {
        diagram_type_filter: args.diagram_type.clone(),
        skip_visual: true, // TODO: enable when PNG rendering is ready
        force_refresh: args.force_refresh,
        ..Default::default()
    };

    let cache = eval::cache::ReferenceCache::with_defaults();
    let runner = eval::runner::EvalRunner::new(eval_config, cache);

    // Get diagrams to evaluate
    let inputs = match &args.target {
        None => {
            // Use built-in gallery samples
            eprintln!("Using built-in gallery samples...");
            samples::all_samples()
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

    // Output results
    if args.verbose {
        println!("{}", eval::report::text_detailed(&result));
    } else {
        println!("{}", eval::report::text_summary(&result));
    }

    // Write JSON if requested
    if let Some(ref path) = args.output {
        eval::report::write_json(&result, path)?;
        eprintln!("Wrote JSON report to {}", path.display());
    }

    // Write HTML if requested
    if let Some(ref path) = args.html {
        eval::report::write_html(&result, path)?;
        eprintln!("Wrote HTML report to {}", path.display());
    }

    // Write PNGs if requested
    if let Some(ref _path) = args.pngs {
        eprintln!("PNG generation not yet implemented (requires 'png' feature)");
        // TODO: Implement when resvg is available
    }

    // Exit with error code if there are failures
    if result.issue_counts.errors > 0 {
        process::exit(1);
    }

    Ok(())
}

/// Load diagrams from a directory of .mmd files
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
    opt.fontdb_mut().load_system_fonts();

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
    opt.fontdb_mut().load_system_fonts();

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
