//! Colorized console helpers.

use std::env;
use std::io::{self, IsTerminal, Write};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

/// Output stream variants for color handling and fallbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stream {
    Stdout,
    Stderr,
}

/// Global strategy for deciding whether ANSI colors should be emitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Auto = 0,
    Always = 1,
    Never = 2,
}

static COLOR_MODE: AtomicU8 = AtomicU8::new(ColorMode::Auto as u8);
static ENV_DISABLE_COLOR: OnceLock<bool> = OnceLock::new();

/// Read the active color mode.
pub fn color_mode() -> ColorMode {
    match COLOR_MODE.load(Ordering::Relaxed) {
        0 => ColorMode::Auto,
        1 => ColorMode::Always,
        2 => ColorMode::Never,
        _ => {
            COLOR_MODE.store(ColorMode::Auto as u8, Ordering::Relaxed);
            ColorMode::Auto
        }
    }
}

/// Set the active color mode.
pub fn set_color_mode(mode: ColorMode) {
    COLOR_MODE.store(mode as u8, Ordering::Relaxed);
}

/// Force-enable colors regardless of terminal detection.
pub fn enable_color() {
    set_color_mode(ColorMode::Always);
}

/// Disable colors for all subsequent console output.
pub fn disable_color() {
    set_color_mode(ColorMode::Never);
}

/// Reset color behaviour back to auto detection.
pub fn reset_color_mode() {
    set_color_mode(ColorMode::Auto);
}

/// Test whether ANSI colors should be emitted for a given stream.
pub fn color_enabled(stream: Stream) -> bool {
    match color_mode() {
        ColorMode::Always => true,
        ColorMode::Never => false,
        ColorMode::Auto => {
            if env_disables_color() {
                return false;
            }
            match stream {
                Stream::Stdout => io::stdout().is_terminal(),
                Stream::Stderr => io::stderr().is_terminal(),
            }
        }
    }
}

fn env_disables_color() -> bool {
    *ENV_DISABLE_COLOR.get_or_init(|| {
        env::var_os("NO_COLOR").is_some()
            || env::var("TERM")
                .map(|term| term.eq_ignore_ascii_case("dumb"))
                .unwrap_or(false)
    })
}

fn paint(text: &str, code: &str, stream: Stream) -> String {
    if code.is_empty() || !color_enabled(stream) {
        return text.to_string();
    }
    format!("\x1b[{code}m{text}\x1b[0m")
}

/// Apply ANSI color/style to a string for stdout.
pub fn color(text: &str, code: &str) -> String {
    paint(text, code, Stream::Stdout)
}

/// Apply ANSI color/style to a string for a specific stream.
pub fn color_for_stream(text: &str, code: &str, stream: Stream) -> String {
    paint(text, code, stream)
}

/// Apply multiple ANSI style codes to a string for stdout.
pub fn style(text: &str, codes: &[&str]) -> String {
    style_for_stream(text, codes, Stream::Stdout)
}

/// Apply multiple ANSI style codes for a specific stream.
pub fn style_for_stream(text: &str, codes: &[&str], stream: Stream) -> String {
    let joined = codes.join(";");
    paint(text, &joined, stream)
}

/// Highlight text using bold and underline styles.
pub fn highlight(text: &str) -> String {
    style(text, &["1", "4"])
}

/// Render text in a dimmed style.
pub fn dim(text: &str) -> String {
    style(text, &["2"])
}

/// Render text in bold.
pub fn bold(text: &str) -> String {
    style(text, &["1"])
}

/// Render text in green (for success).
pub fn green(text: &str) -> String {
    color_for_stream(text, "32", Stream::Stderr)
}

/// Render text in red (for errors).
pub fn red(text: &str) -> String {
    color_for_stream(text, "1;91", Stream::Stderr)
}

/// Render text in yellow (for warnings).
pub fn yellow(text: &str) -> String {
    color_for_stream(text, "1;93", Stream::Stderr)
}

/// Render text in cyan.
pub fn cyan(text: &str) -> String {
    color_for_stream(text, "36", Stream::Stderr)
}

/// Render text with underline style.
pub fn underline(text: &str) -> String {
    style(text, &["4"])
}

/// Format a duration in a human-friendly way.
///
/// - Less than 1s: shows seconds with precision, clamped to 0.001s (e.g., "0.123s", "0.001s")
/// - 1s to 59s: shows seconds with one decimal (e.g., "5.2s")
/// - 60s to 59m59s: shows minutes and seconds (e.g., "1m 32s", "5m 0s")
/// - 1h+: shows hours and minutes (e.g., "1h 5m", "2h 30m")
pub fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let millis = duration.subsec_millis();

    if total_secs == 0 {
        // sub-second: show as decimal seconds
        let secs_f = duration.as_secs_f64();
        if secs_f == 0.0 {
            "0s".to_string()
        } else if secs_f < 0.001 {
            "0.001s".to_string()
        } else if secs_f < 0.01 {
            // very small: 3 decimal places
            format!("{secs_f:.3}s")
        } else if secs_f < 0.1 {
            // small: 2 decimal places
            format!("{secs_f:.2}s")
        } else {
            // larger sub-second: 1 decimal
            format!("{secs_f:.1}s")
        }
    } else if total_secs < 60 {
        // under a minute: seconds with one decimal
        let secs_f = duration.as_secs_f64();
        if millis == 0 {
            format!("{total_secs}s")
        } else {
            format!("{secs_f:.1}s")
        }
    } else if total_secs < 3600 {
        // under an hour: minutes and seconds
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        format!("{mins}m {secs}s")
    } else {
        // hours and minutes
        let hours = total_secs / 3600;
        let mins = (total_secs % 3600) / 60;
        format!("{hours}h {mins}m")
    }
}

/// Format a duration with color based on how long it is.
///
/// - Green: under 1s (fast)
/// - Yellow: 1s to 10s (moderate)
/// - Red: over 10s (slow)
pub fn format_duration_colored(duration: Duration) -> String {
    let text = format_duration(duration);
    let secs = duration.as_secs_f64();

    if secs < 1.0 {
        green(&text)
    } else if secs < 10.0 {
        yellow(&text)
    } else {
        red(&text)
    }
}

/// Print a line to stdout.
pub fn print(text: &str) {
    println!("{text}");
}

/// Print an info line to stdout in cyan.
pub fn info(text: &str) {
    println!("{}", color(text, "36"));
}

/// Print a success line to stdout in green.
pub fn success(text: &str) {
    println!("{}", color(text, "32"));
}

/// Print a warning line to stdout in yellow.
pub fn warn(text: &str) {
    println!("{}", color(text, "33"));
}

/// Print a debug line to stderr in magenta.
pub fn debug(text: &str) {
    eprintln!("{}", color_for_stream(text, "35", Stream::Stderr));
}

/// Print an error line to stderr in red.
pub fn error(text: &str) {
    eprintln!("{}", color_for_stream(text, "31", Stream::Stderr));
}

/// Print text without newline to stdout.
pub fn write(text: &str) {
    let mut handle = io::stdout();
    let _ = handle.write_all(text.as_bytes());
    let _ = handle.flush();
}

/// Print text with newline to stdout.
pub fn write_line(text: &str) {
    let mut handle = io::stdout();
    let _ = handle.write_all(text.as_bytes());
    let _ = handle.write_all(b"\n");
    let _ = handle.flush();
}

/// Gather free-form input from stdin after printing a prompt.
pub fn prompt_input(prompt: &str) -> Result<String, String> {
    write(prompt);
    let mut buffer = String::new();
    io::stdin()
        .read_line(&mut buffer)
        .map_err(|error| format!("failed to read input: {error}"))?;
    let trimmed = buffer.trim_end_matches(['\n', '\r']).to_string();
    Ok(trimmed)
}

/// Ask the user for confirmation, returning the default on empty input.
pub fn prompt_yes_no(question: &str, default: bool) -> Result<bool, String> {
    loop {
        let suffix = if default { " [Y/n] " } else { " [y/N] " };
        let response = prompt_input(&format!("{question}{suffix}"))?;
        let normalized = response.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            return Ok(default);
        }
        if matches!(normalized.as_str(), "y" | "yes") {
            return Ok(true);
        }
        if matches!(normalized.as_str(), "n" | "no") {
            return Ok(false);
        }
        warn("Please answer yes or no");
    }
}

/// Create a header rendered with an underline that respects ANSI codes.
pub fn header(text: &str, underline: char) -> String {
    let width = visible_width(text);
    let line = underline.to_string().repeat(width);
    format!("{line}\n{text}\n{line}")
}

/// Render a simple ASCII/Unicode frame around content with optional title.
pub fn frame(content: &str, title: Option<&str>, padding: u8) -> String {
    // split content into lines
    let mut lines: Vec<&str> = if content.is_empty() {
        vec![""]
    } else {
        content.split('\n').collect()
    };

    // drop trailing empty line if content ended with a newline
    if content.ends_with('\n') && lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }

    // count max visible width of content lines plus padding
    let content_width = lines
        .iter()
        .map(|line| visible_width(line))
        .max()
        .unwrap_or(0);
    let inner_width = content_width + (padding as usize * 2);
    let mut out = String::new();

    // top border sized to inner width
    if let Some(title_text) = title.filter(|title| !title.is_empty()) {
        let prefix = format!("─ {title_text} ");
        let used = prefix.chars().count();
        let fill = inner_width.saturating_sub(used);
        out.push('┌');
        out.push_str(&prefix);
        out.push_str(&"─".repeat(fill));
        out.push('┐');
    } else {
        out.push_str(&format!("┌{}┐", "─".repeat(inner_width)));
    }
    out.push('\n');

    // content lines with padding
    let pad_str = " ".repeat(padding as usize);
    for line in lines {
        let line_visible = visible_width(line);
        let content_pad = inner_width.saturating_sub(line_visible + (padding as usize * 2));
        out.push('│');
        out.push_str(&pad_str);
        out.push_str(line);
        out.push_str(&" ".repeat(content_pad));
        out.push_str(&pad_str);
        out.push('│');
        out.push('\n');
    }

    // bottom border
    out.push_str(&format!("└{}┘", "─".repeat(inner_width)));
    out
}

/// Remove ANSI escape sequences from a string.
pub fn strip_ansi_codes(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            let _ = chars.next();
            for code_char in chars.by_ref() {
                if code_char == 'm' {
                    break;
                }
            }
            continue;
        }
        out.push(ch);
    }
    out
}

/// Calculate the visible width of a string, ignoring ANSI escape sequences.
pub fn visible_width(text: &str) -> usize {
    strip_ansi_codes(text).chars().count()
}

/// Page content through `less -R -S -F -X` for colored output and better UX.
pub fn page_with_less(content: &str) -> Result<(), String> {
    let mut child = Command::new("less")
        .arg("-R")
        .arg("-S")
        .arg("-F")
        .arg("-X")
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to spawn less: {error}"))?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(content.as_bytes())
            .map_err(|error| format!("failed to write to less stdin: {error}"))?;
    }

    let status = child
        .wait()
        .map_err(|error| format!("failed to wait for less: {error}"))?;
    if !status.success() {
        return Err(format!("less exited with status {status}"));
    }
    Ok(())
}

/// Page content when possible and fall back to printing directly.
pub fn page_or_print(content: &str) -> Result<(), String> {
    match page_with_less(content) {
        Ok(()) => Ok(()),
        Err(error) => {
            debug(&format!("falling back to inline print: {error}"));
            print(content);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visible_width_ignores_ansi() {
        let sample = "plain";
        assert_eq!(visible_width(sample), 5);
        let colored = color(sample, "31");
        assert_eq!(visible_width(&colored), 5);
    }

    #[test]
    fn test_color_mode_toggle() {
        disable_color();
        assert!(!color_enabled(Stream::Stdout));

        enable_color();
        assert!(color_enabled(Stream::Stdout));

        reset_color_mode();
    }

    #[test]
    fn test_format_duration_subsecond() {
        assert_eq!(format_duration(Duration::from_millis(1)), "0.001s");
        assert_eq!(format_duration(Duration::from_millis(5)), "0.005s");
        assert_eq!(format_duration(Duration::from_millis(12)), "0.01s");
        assert_eq!(format_duration(Duration::from_millis(123)), "0.1s");
        assert_eq!(format_duration(Duration::from_millis(500)), "0.5s");
        assert_eq!(format_duration(Duration::from_millis(999)), "1.0s");
    }

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration(Duration::from_secs(1)), "1s");
        assert_eq!(format_duration(Duration::from_millis(1500)), "1.5s");
        assert_eq!(format_duration(Duration::from_secs(30)), "30s");
        assert_eq!(format_duration(Duration::from_secs(59)), "59s");
    }

    #[test]
    fn test_format_duration_minutes() {
        assert_eq!(format_duration(Duration::from_secs(60)), "1m 0s");
        assert_eq!(format_duration(Duration::from_secs(92)), "1m 32s");
        assert_eq!(format_duration(Duration::from_secs(300)), "5m 0s");
        assert_eq!(format_duration(Duration::from_secs(3599)), "59m 59s");
    }

    #[test]
    fn test_format_duration_hours() {
        assert_eq!(format_duration(Duration::from_secs(3600)), "1h 0m");
        assert_eq!(format_duration(Duration::from_secs(3900)), "1h 5m");
        assert_eq!(format_duration(Duration::from_secs(9000)), "2h 30m");
    }
}
