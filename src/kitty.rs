//! Display images in terminals using the Kitty graphics protocol.
//!
//! The Kitty graphics protocol allows displaying pixel graphics directly
//! in the terminal. This module provides functions to display PNG images
//! using the protocol.
//!
//! Supported terminals:
//! - [Kitty](https://sw.kovidgoyal.net/kitty/)
//! - [Ghostty](https://ghostty.org/)
//!
//! # References
//! - <https://sw.kovidgoyal.net/kitty/graphics-protocol/>

use std::io::{self, Read, Write};
use std::sync::OnceLock;

use base64::Engine;

/// Cached result of kitty support detection.
static KITTY_SUPPORT: OnceLock<bool> = OnceLock::new();

/// Check if the current terminal supports the Kitty graphics protocol.
///
/// Uses the Kitty graphics protocol query mechanism to detect support.
/// Falls back to environment variable checks if the query fails.
///
/// The result is cached after the first call.
pub fn is_supported() -> bool {
    *KITTY_SUPPORT.get_or_init(|| {
        // Try query-based detection first
        if query_kitty_support() {
            return true;
        }
        // Fall back to environment variable checks
        check_env_for_support()
    })
}

/// Query the terminal for Kitty graphics protocol support.
///
/// Sends a test query and checks for a valid response.
#[cfg(unix)]
fn query_kitty_support() -> bool {
    use std::os::unix::io::AsRawFd;

    // Only works with a real TTY
    if !atty::is(atty::Stream::Stdin) || !atty::is(atty::Stream::Stdout) {
        return false;
    }

    let stdin = io::stdin();
    let fd = stdin.as_raw_fd();

    // Save terminal settings
    let old_termios = match termios_get(fd) {
        Some(t) => t,
        None => return false,
    };

    let result = (|| {
        // Set terminal to raw mode
        if !termios_set_raw(fd) {
            return false;
        }

        // Send kitty graphics query
        // a=q means query, i=31 is a test image id, s/v=1 is 1x1 pixel
        let query = "\x1b_Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\";
        if io::stdout().write_all(query.as_bytes()).is_err() {
            return false;
        }
        if io::stdout().flush().is_err() {
            return false;
        }

        // Read response with timeout
        let response = read_with_timeout(fd, 100);

        // Check for valid kitty graphics response
        response.contains("_G") && response.contains("i=31")
    })();

    // Restore terminal settings
    termios_restore(fd, &old_termios);

    result
}

#[cfg(not(unix))]
fn query_kitty_support() -> bool {
    false
}

/// Check environment variables for Kitty graphics support hints.
fn check_env_for_support() -> bool {
    // Check TERM environment variable
    if let Ok(term) = std::env::var("TERM") {
        let term_lower = term.to_lowercase();
        if term_lower.contains("kitty") || term_lower.contains("ghostty") {
            return true;
        }
    }

    // Check TERM_PROGRAM
    if let Ok(term_program) = std::env::var("TERM_PROGRAM") {
        let tp_lower = term_program.to_lowercase();
        if tp_lower.contains("kitty") || tp_lower.contains("ghostty") {
            return true;
        }
    }

    // Check for kitty-specific environment
    if std::env::var("KITTY_WINDOW_ID").is_ok() {
        return true;
    }

    // Check for Ghostty-specific environment
    if std::env::var("GHOSTTY_RESOURCES_DIR").is_ok() {
        return true;
    }

    false
}

/// Terminal background color as RGB values (0-255).
#[derive(Debug, Clone, Copy)]
pub struct BackgroundColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl BackgroundColor {
    /// Calculate relative luminance using ITU-R BT.709 formula.
    pub fn luminance(&self) -> f32 {
        0.2126 * (self.r as f32) + 0.7152 * (self.g as f32) + 0.0722 * (self.b as f32)
    }

    /// Check if this background color is considered "dark".
    ///
    /// Uses luminance threshold of ~40% (102/255).
    pub fn is_dark(&self) -> bool {
        self.luminance() < 102.0
    }
}

/// Get the terminal's background color.
///
/// Tries OSC 11 query first, then falls back to reading config files.
#[cfg(unix)]
pub fn get_terminal_background() -> Option<BackgroundColor> {
    // Try OSC 11 query first
    if let Some(bg) = query_terminal_color(11) {
        return Some(bg);
    }

    // Try reading from Ghostty config if in Ghostty
    if std::env::var("GHOSTTY_RESOURCES_DIR").is_ok() {
        if let Some(bg) = get_ghostty_background() {
            return Some(bg);
        }
    }

    None
}

#[cfg(not(unix))]
pub fn get_terminal_background() -> Option<BackgroundColor> {
    None
}

/// Detect if the terminal is using a dark color scheme.
///
/// Uses OSC 11 to query background color and calculates luminance.
/// Falls back to environment variables and config files.
pub fn is_terminal_dark() -> bool {
    // Try querying the actual background color
    if let Some(bg) = get_terminal_background() {
        return bg.is_dark();
    }

    // Fall back to COLORFGBG environment variable
    if let Ok(colorfgbg) = std::env::var("COLORFGBG") {
        let parts: Vec<&str> = colorfgbg.split(';').collect();
        if parts.len() >= 2 {
            if let Ok(bg_color) = parts.last().unwrap_or(&"").parse::<u8>() {
                // 0 = black (dark), 15 = white (light)
                return bg_color < 8;
            }
        }
    }

    // Default: assume dark (more common for terminal users)
    true
}

/// Query terminal for a color using OSC escape sequence.
#[cfg(unix)]
fn query_terminal_color(osc_code: u8) -> Option<BackgroundColor> {
    use std::os::unix::io::AsRawFd;

    if !atty::is(atty::Stream::Stdin) || !atty::is(atty::Stream::Stdout) {
        return None;
    }

    let stdin = io::stdin();
    let fd = stdin.as_raw_fd();

    let old_termios = termios_get(fd)?;

    let result = (|| {
        if !termios_set_raw(fd) {
            return None;
        }

        // Query color: OSC 11 = background
        let query = format!("\x1b]{};?\x1b\\", osc_code);
        io::stdout().write_all(query.as_bytes()).ok()?;
        io::stdout().flush().ok()?;

        let response = read_with_timeout(fd, 100);

        // Parse response: OSC 11 ; rgb:RRRR/GGGG/BBBB ST
        parse_rgb_response(&response)
    })();

    termios_restore(fd, &old_termios);
    result
}

/// Parse RGB color from terminal response.
fn parse_rgb_response(response: &str) -> Option<BackgroundColor> {
    // Look for rgb:RRRR/GGGG/BBBB pattern
    let rgb_start = response.find("rgb:")?;
    let rgb_part = &response[rgb_start + 4..];

    let parts: Vec<&str> = rgb_part.split('/').take(3).collect();
    if parts.len() < 3 {
        return None;
    }

    // Clean up the last part (remove trailing escape sequences)
    let b_part = parts[2]
        .chars()
        .take_while(|c| c.is_ascii_hexdigit())
        .collect::<String>();

    // Convert 16-bit hex to 8-bit (take first 2 hex digits)
    let r = u8::from_str_radix(&parts[0].chars().take(2).collect::<String>(), 16).ok()?;
    let g = u8::from_str_radix(&parts[1].chars().take(2).collect::<String>(), 16).ok()?;
    let b = u8::from_str_radix(&b_part.chars().take(2).collect::<String>(), 16).ok()?;

    Some(BackgroundColor { r, g, b })
}

/// Try to read background color from Ghostty config file.
fn get_ghostty_background() -> Option<BackgroundColor> {
    let home = std::env::var("HOME").ok()?;
    let config_path = format!("{}/.config/ghostty/config", home);

    let content = std::fs::read_to_string(&config_path).ok()?;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("background") {
            if let Some(value) = line.split('=').nth(1) {
                let value = value.trim();
                if value.starts_with('#') && value.len() >= 7 {
                    let r = u8::from_str_radix(&value[1..3], 16).ok()?;
                    let g = u8::from_str_radix(&value[3..5], 16).ok()?;
                    let b = u8::from_str_radix(&value[5..7], 16).ok()?;
                    return Some(BackgroundColor { r, g, b });
                }
            }
        }
    }

    None
}

/// Display PNG image data in the terminal using Kitty graphics protocol.
///
/// The PNG data is converted to RGBA format and transmitted in chunks.
///
/// # Arguments
/// * `png_data` - Raw PNG image data (not base64 encoded)
///
/// # Errors
/// Returns an error if the image cannot be decoded or displayed.
pub fn display_png(png_data: &[u8]) -> Result<(), DisplayError> {
    // Decode PNG to RGBA
    let img =
        image::load_from_memory(png_data).map_err(|e| DisplayError::ImageDecode(e.to_string()))?;

    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let raw_data = rgba.into_raw();

    // Base64 encode
    let encoded = base64::engine::general_purpose::STANDARD.encode(&raw_data);

    // Send via chunked protocol
    write_chunked_rgba(&encoded, width, height)?;

    // Add newline after image
    println!();

    Ok(())
}

/// Write RGBA image data in chunks using Kitty graphics protocol.
fn write_chunked_rgba(data: &str, width: u32, height: u32) -> Result<(), DisplayError> {
    const CHUNK_SIZE: usize = 4096;

    let mut remaining = data;
    let mut first_chunk = true;
    let mut stdout = io::stdout().lock();

    while !remaining.is_empty() {
        let (chunk, rest) = if remaining.len() > CHUNK_SIZE {
            remaining.split_at(CHUNK_SIZE)
        } else {
            (remaining, "")
        };
        remaining = rest;

        // m=1 means more data coming, m=0 means final chunk
        let more = if remaining.is_empty() { "0" } else { "1" };

        if first_chunk {
            // First chunk: a=T (transmit+display), f=32 (RGBA), s=width, v=height
            write!(
                stdout,
                "\x1b_Ga=T,f=32,s={},v={},t=d,m={};{}\x1b\\",
                width, height, more, chunk
            )
            .map_err(|e| DisplayError::Write(e.to_string()))?;
            first_chunk = false;
        } else {
            // Subsequent chunks only need the more flag
            write!(stdout, "\x1b_Gm={};{}\x1b\\", more, chunk)
                .map_err(|e| DisplayError::Write(e.to_string()))?;
        }
    }

    stdout
        .flush()
        .map_err(|e| DisplayError::Write(e.to_string()))?;
    Ok(())
}

/// Clear all images from the terminal screen.
pub fn clear_images() {
    // a=d means delete, d=A means all images
    let _ = io::stdout().write_all(b"\x1b_Ga=d,d=A\x1b\\");
    let _ = io::stdout().flush();
}

/// Errors that can occur when displaying images.
#[derive(Debug)]
pub enum DisplayError {
    /// Failed to decode the image
    ImageDecode(String),
    /// Failed to write to stdout
    Write(String),
}

impl std::fmt::Display for DisplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisplayError::ImageDecode(e) => write!(f, "Failed to decode image: {}", e),
            DisplayError::Write(e) => write!(f, "Failed to write to terminal: {}", e),
        }
    }
}

impl std::error::Error for DisplayError {}

// ============================================================================
// Unix terminal handling
// ============================================================================

#[cfg(unix)]
use libc::{c_int, tcgetattr, tcsetattr, termios, ECHO, ICANON, TCSADRAIN};

#[cfg(unix)]
fn termios_get(fd: c_int) -> Option<termios> {
    unsafe {
        let mut t: termios = std::mem::zeroed();
        if tcgetattr(fd, &mut t) == 0 {
            Some(t)
        } else {
            None
        }
    }
}

#[cfg(unix)]
fn termios_set_raw(fd: c_int) -> bool {
    unsafe {
        let mut t: termios = std::mem::zeroed();
        if tcgetattr(fd, &mut t) != 0 {
            return false;
        }
        // Disable canonical mode and echo
        t.c_lflag &= !(ICANON | ECHO);
        tcsetattr(fd, TCSADRAIN, &t) == 0
    }
}

#[cfg(unix)]
fn termios_restore(fd: c_int, t: &termios) {
    unsafe {
        tcsetattr(fd, TCSADRAIN, t);
    }
}

#[cfg(unix)]
fn read_with_timeout(fd: c_int, timeout_ms: u64) -> String {
    use std::time::{Duration, Instant};

    let mut response = String::new();
    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);

    // Set non-blocking read
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL);
        libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
    }

    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut buf = [0u8; 1];

    while start.elapsed() < timeout {
        match handle.read(&mut buf) {
            Ok(1) => {
                response.push(buf[0] as char);
                if buf[0] == b'\\' {
                    break;
                }
            }
            Ok(_) => break,
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(_) => break,
        }
    }

    // Restore blocking mode
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL);
        libc::fcntl(fd, libc::F_SETFL, flags & !libc::O_NONBLOCK);
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_background_color_luminance() {
        // Black should have zero luminance
        let black = BackgroundColor { r: 0, g: 0, b: 0 };
        assert!(black.luminance() < 1.0);

        // White should have high luminance
        let white = BackgroundColor {
            r: 255,
            g: 255,
            b: 255,
        };
        assert!(white.luminance() > 250.0);

        // Pure green has highest weight in luminance formula
        let green = BackgroundColor { r: 0, g: 255, b: 0 };
        let red = BackgroundColor { r: 255, g: 0, b: 0 };
        assert!(green.luminance() > red.luminance());
    }

    #[test]
    fn test_background_color_is_dark() {
        // Black is dark
        let black = BackgroundColor { r: 0, g: 0, b: 0 };
        assert!(black.is_dark());

        // White is not dark
        let white = BackgroundColor {
            r: 255,
            g: 255,
            b: 255,
        };
        assert!(!white.is_dark());

        // Typical dark theme background (#1e1e1e)
        let dark_bg = BackgroundColor {
            r: 30,
            g: 30,
            b: 30,
        };
        assert!(dark_bg.is_dark());

        // Typical light theme background (#f5f5f5)
        let light_bg = BackgroundColor {
            r: 245,
            g: 245,
            b: 245,
        };
        assert!(!light_bg.is_dark());
    }

    #[test]
    fn test_parse_rgb_response_standard() {
        // Standard 16-bit response format
        let response = "\x1b]11;rgb:1e1e/1e1e/1e1e\x1b\\";
        let color = parse_rgb_response(response).unwrap();
        assert_eq!(color.r, 0x1e);
        assert_eq!(color.g, 0x1e);
        assert_eq!(color.b, 0x1e);
    }

    #[test]
    fn test_parse_rgb_response_white() {
        let response = "\x1b]11;rgb:ffff/ffff/ffff\x1b\\";
        let color = parse_rgb_response(response).unwrap();
        assert_eq!(color.r, 0xff);
        assert_eq!(color.g, 0xff);
        assert_eq!(color.b, 0xff);
    }

    #[test]
    fn test_parse_rgb_response_invalid() {
        // No rgb: prefix
        assert!(parse_rgb_response("invalid response").is_none());

        // Incomplete
        assert!(parse_rgb_response("rgb:ff/ff").is_none());
    }

    #[test]
    fn test_display_error_display() {
        let decode_err = DisplayError::ImageDecode("test error".to_string());
        assert!(decode_err.to_string().contains("decode"));

        let write_err = DisplayError::Write("write failed".to_string());
        assert!(write_err.to_string().contains("write"));
    }
}
