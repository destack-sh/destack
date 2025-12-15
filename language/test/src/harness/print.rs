use std::time::Duration;

use super::{TestCase, TestResult, TestSummary};

/// ANSI color codes.
pub mod color {
    pub const BOLD: &str = "\x1b[1m";
    pub const GREEN: &str = "\x1b[32m";
    pub const RED: &str = "\x1b[31m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const CYAN: &str = "\x1b[36m";
    pub const DIM: &str = "\x1b[2m";
    pub const RESET: &str = "\x1b[0m";

    /// Apply bold green.
    pub fn green(s: &str) -> String {
        format!("{BOLD}{GREEN}{s}{RESET}")
    }

    /// Apply bold red.
    pub fn red(s: &str) -> String {
        format!("{BOLD}{RED}{s}{RESET}")
    }

    /// Apply bold yellow.
    pub fn yellow(s: &str) -> String {
        format!("{BOLD}{YELLOW}{s}{RESET}")
    }

    /// Apply bold cyan.
    pub fn cyan(s: &str) -> String {
        format!("{BOLD}{CYAN}{s}{RESET}")
    }

    /// Apply dim.
    pub fn dim(s: &str) -> String {
        format!("{DIM}{s}{RESET}")
    }

    /// Apply bold.
    pub fn bold(s: &str) -> String {
        format!("{BOLD}{s}{RESET}")
    }
}

/// Compute the visible width of a string, ignoring ANSI SGR color sequences.
///
/// This is used to pad boxed output so the right border lines up.
fn visible_width(text: &str) -> usize {
    let mut width = 0;
    let mut bytes = text.as_bytes();
    while !bytes.is_empty() {
        if bytes[0] == 0x1b && bytes.get(1) == Some(&b'[') {
            // skip ANSI escape sequences like "\x1b[...m"
            let mut i = 2;
            while i < bytes.len() {
                if bytes[i] == b'm' {
                    i += 1;
                    break;
                }
                i += 1;
            }
            bytes = &bytes[i.min(bytes.len())..];
            continue;
        }

        // decode the next utf8 character
        let next_char = match std::str::from_utf8(bytes) {
            Ok(s) => s.chars().next(),
            Err(err) => {
                // skip invalid bytes to keep the renderer resilient
                bytes = &bytes[err.valid_up_to().saturating_add(1)..];
                continue;
            }
        };

        if let Some(ch) = next_char {
            width += 1;
            bytes = &bytes[ch.len_utf8()..];
        } else {
            break;
        }
    }

    width
}

/// Print test result with colors.
pub fn print_result(test: &TestCase, result: &TestResult, duration: Duration, _verbose: bool) {
    let (_status_plain, status) = match result {
        TestResult::Passed => ("ok".to_string(), color::green("ok")),
        TestResult::Failed { .. } => ("FAILED".to_string(), color::red("FAILED")),
        TestResult::Skipped { .. } => ("skipped".to_string(), color::yellow("skipped")),
        TestResult::Suite { passed, failed, .. } => {
            if *failed == 0 {
                let plain = format!("suite ok ({passed} passed)");
                let colored = color::green(&plain);
                (plain, colored)
            } else {
                let plain = format!("suite ({passed} passed, {failed} failed)");
                let colored = color::yellow(&plain);
                (plain, colored)
            }
        }
    };

    let duration_plain = if duration.as_millis() > 100 {
        format!(" ({:.2}s)", duration.as_secs_f64())
    } else {
        String::new()
    };
    let duration_str = if duration_plain.is_empty() {
        String::new()
    } else {
        color::dim(&duration_plain)
    };

    println!("test {} ... {}{}", test.full_name(), status, duration_str);

    if let TestResult::Failed { message } = result {
        if message.is_empty() {
            return;
        }

        let indent = "       ";

        // calculate max visible width of all content lines
        let max_content_width = message
            .lines()
            .map(|line| visible_width(line))
            .max()
            .unwrap_or(0);

        // box inner width must fit the widest content line (plus minimum of 8)
        let inner_width = max_content_width.max(8);
        // separator includes: │ + space + content + space + │
        let separator_len = inner_width + 4;

        let border_top = color::dim(&format!(
            "┌{}┐",
            "─".repeat(separator_len.saturating_sub(2))
        ));
        let border_bottom = color::dim(&format!(
            "└{}┘",
            "─".repeat(separator_len.saturating_sub(2))
        ));
        let border_left = color::dim("│");
        let border_right = color::dim("│");

        println!("{indent}{border_top}");

        for line in message.lines() {
            let visible = visible_width(line);
            let padding = inner_width.saturating_sub(visible);
            println!(
                "{indent}{border_left} {line}{} {border_right}",
                " ".repeat(padding)
            );
        }

        println!("{indent}{border_bottom}");
        println!();
    }
}

/// Print test summary.
pub fn print_summary(summary: &TestSummary, duration: Duration) {
    let passed = summary.passed();
    let failed = summary.failed();
    let skipped = summary.skipped();

    let status = if failed == 0 {
        color::green("ok")
    } else {
        color::red("FAILED")
    };

    // build parts with colors
    let mut parts = Vec::new();
    if passed > 0 {
        parts.push(color::green(&format!("{passed} passed")));
    }
    if failed > 0 {
        parts.push(color::red(&format!("{failed} failed")));
    }
    if skipped > 0 {
        parts.push(color::yellow(&format!("{skipped} skipped")));
    }

    let counts = parts.join("; ");
    let time = color::dim(&format!("finished in {:.2}s", duration.as_secs_f64()));

    println!();
    println!("test result: {status}. {counts}; {time}");
}

/// Print failures summary (list of failed test names).
pub fn print_failures(tests: &[(TestCase, TestResult)]) {
    let failures: Vec<_> = tests.iter().filter(|(_, r)| r.is_failed()).collect();
    if failures.is_empty() {
        return;
    }

    // just list failed test names (diagnostics were already printed inline)
    println!();
    println!("{}:", color::red("failures"));
    for (test, _) in &failures {
        println!("    {}", test.full_name());
    }
}

/// Print test list without running.
pub fn print_test_list(tests: &[TestCase]) {
    for test in tests {
        let suffix = if test.is_skipped {
            format!(": {} {}", color::yellow("test"), color::dim("(skipped)"))
        } else {
            format!(": {}", color::cyan("test"))
        };
        println!("{}{}", test.full_name(), suffix);
    }

    println!();
    let total = color::bold(&format!("{}", tests.len()));
    let skipped_count = tests.iter().filter(|t| t.is_skipped).count();
    if skipped_count > 0 {
        let skipped = color::yellow(&format!("{skipped_count} skipped"));
        println!("{total} tests ({skipped})");
    } else {
        println!("{total} tests");
    }
}
