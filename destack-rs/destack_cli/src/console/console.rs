//! Colorized console helpers.

use std::io::{self, Write};

/// Apply ANSI color/style to a string.
pub fn color(text: &str, code: &str) -> String {
    format!("\x1b[{code}m{text}\x1b[0m")
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

/// Print an error line to stderr in red.
pub fn error(text: &str) {
    eprintln!("{}", color(text, "31"));
}

/// Print text without newline to stdout.
pub fn write(text: &str) {
    let _ = io::stdout().write_all(text.as_bytes());
    let _ = io::stdout().flush();
}

/// Create a header rendered with an underline.
pub fn header(text: &str, underline: char) -> String {
    let line = underline.to_string().repeat(text.len());
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
    if content.ends_with('\n') && lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }

    // count max width of content lines plus padding
    let content_width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let inner_width = content_width + (padding as usize * 2);
    let mut out = String::new();

    // top border sized to inner width
    if let Some(t) = title.filter(|t| !t.is_empty()) {
        let prefix = format!("─ {t} ");
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
    for l in lines {
        let content_pad = inner_width.saturating_sub(l.chars().count() + (padding as usize * 2));
        out.push('│');
        out.push_str(&pad_str);
        out.push_str(l);
        out.push_str(&" ".repeat(content_pad));
        out.push_str(&pad_str);
        out.push('│');
        out.push('\n');
    }

    // bottom border
    out.push_str(&format!("└{}┘", "─".repeat(inner_width)));
    out
}
