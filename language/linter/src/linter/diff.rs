/// ANSI color codes.
#[allow(dead_code)]
mod color {
    const BOLD: &str = "\x1b[1m";
    const GREEN: &str = "\x1b[32m";
    const RED: &str = "\x1b[31m";
    const YELLOW: &str = "\x1b[33m";
    const CYAN: &str = "\x1b[36m";
    const DIM: &str = "\x1b[2m";
    const RESET: &str = "\x1b[0m";

    /// Apply bold green.
    pub(super) fn green(s: &str) -> String {
        format!("{BOLD}{GREEN}{s}{RESET}")
    }

    /// Apply bold red.
    pub(super) fn red(s: &str) -> String {
        format!("{BOLD}{RED}{s}{RESET}")
    }

    /// Apply bold yellow.
    pub(super) fn yellow(s: &str) -> String {
        format!("{BOLD}{YELLOW}{s}{RESET}")
    }

    /// Apply bold cyan.
    pub(super) fn cyan(s: &str) -> String {
        format!("{BOLD}{CYAN}{s}{RESET}")
    }

    /// Apply dim.
    pub(super) fn dim(s: &str) -> String {
        format!("{DIM}{s}{RESET}")
    }

    /// Apply bold.
    pub(super) fn bold(s: &str) -> String {
        format!("{BOLD}{s}{RESET}")
    }
}

/// Print a unified diff between expected and actual content.
pub fn print_diff(expected: &str, actual: &str) {
    eprintln!();

    // show byte lengths for debugging
    eprintln!(
        "  {} bytes: {}, {} bytes: {}",
        color::red("expected"),
        expected.len(),
        color::green("actual"),
        actual.len()
    );

    // check trailing newline differences
    let expected_has_newline = expected.ends_with('\n');
    let actual_has_newline = actual.ends_with('\n');
    if expected_has_newline != actual_has_newline {
        eprintln!(
            "  (trailing newline: {} -> {})",
            if expected_has_newline { "yes" } else { "no" },
            if actual_has_newline { "yes" } else { "no" }
        );
    }

    eprintln!();

    // split into lines
    let expected_lines: Vec<&str> = expected.lines().collect();
    let actual_lines: Vec<&str> = actual.lines().collect();

    // compute simple LCS-based diff
    let mut i = 0;
    let mut j = 0;
    let mut context_before: Vec<(usize, &str)> = Vec::new();
    let context_size = 2;

    while i < expected_lines.len() || j < actual_lines.len() {
        // matching lines
        if i < expected_lines.len()
            && j < actual_lines.len()
            && expected_lines[i] == actual_lines[j]
        {
            context_before.push((i + 1, expected_lines[i]));
            if context_before.len() > context_size {
                context_before.remove(0);
            }
            i += 1;
            j += 1;
            continue;
        }

        // print context before diff
        for (line_no, line) in &context_before {
            eprintln!("{} {}", color::dim(&format!(" {line_no:>4}|")), line);
        }
        context_before.clear();

        // find next matching line
        let mut expected_skip = 0;
        let mut actual_skip = 0;
        'outer: for look_ahead in 1..=10 {
            for ei in 0..=look_ahead {
                let ai = look_ahead - ei;
                if i + ei < expected_lines.len()
                    && j + ai < actual_lines.len()
                    && expected_lines[i + ei] == actual_lines[j + ai]
                {
                    expected_skip = ei;
                    actual_skip = ai;
                    break 'outer;
                }
            }
        }

        // if no match found, consume rest
        if expected_skip == 0 && actual_skip == 0 {
            expected_skip = expected_lines.len().saturating_sub(i);
            actual_skip = actual_lines.len().saturating_sub(j);
        }

        // print removed lines (expected but not in actual)
        for k in 0..expected_skip {
            let line = expected_lines[i + k];
            let escaped = escape_special(line);
            eprintln!("{} {}", color::red(&format!("-{:>4}|", i + k + 1)), escaped);
        }

        // print added lines (in actual but not expected)
        for k in 0..actual_skip {
            let line = actual_lines[j + k];
            let escaped = escape_special(line);
            eprintln!(
                "{} {}",
                color::green(&format!("+{:>4}|", j + k + 1)),
                escaped
            );
        }

        i += expected_skip;
        j += actual_skip;
    }

    // show if files are identical except for whitespace
    if expected.trim() == actual.trim() && expected != actual {
        eprintln!();
        eprintln!(
            "  {}",
            color::yellow("note: files differ only in whitespace")
        );
    }

    eprintln!();
}

/// Escape special characters for display.
pub fn escape_special(s: &str) -> String {
    s.replace('\t', "→").replace('\r', "⏎").replace(' ', "·")
}
