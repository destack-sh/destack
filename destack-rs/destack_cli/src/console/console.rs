//! Colorized console helpers.

use std::io::{self, Write};
use std::process::{Command, Stdio};

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

    // count max visible width of content lines plus padding (ignore ANSI)
    let content_width = lines
        .iter()
        .map(|l| get_ansi_visible_len(l))
        .max()
        .unwrap_or(0);
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
        let line_visible = get_ansi_visible_len(l);
        let content_pad = inner_width.saturating_sub(line_visible + (padding as usize * 2));
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

// strip ANSI escape sequences and compute visible length
fn get_ansi_visible_len(s: &str) -> usize {
    let mut count = 0usize;
    let mut it = s.chars().peekable();
    while let Some(ch) = it.next() {
        if ch == '\u{1b}' && it.peek() == Some(&'[') {
            let _ = it.next(); // skip [
            for c in it.by_ref() {
                if c == 'm' {
                    break;
                }
            }
            continue;
        }
        count += 1;
    }
    count
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
        .map_err(|e| format!("failed to spawn less: {e}"))?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(content.as_bytes())
            .map_err(|e| format!("failed to write to less stdin: {e}"))?;
    }

    let status = child
        .wait()
        .map_err(|e| format!("failed to wait for less: {e}"))?;
    if !status.success() {
        return Err(format!("less exited with status {status}"));
    }
    Ok(())
}
