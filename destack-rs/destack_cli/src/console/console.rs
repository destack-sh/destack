//! Colorized console helpers (no external dependencies).

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
