//! Unified diff output for comparing text.

use destack_base::Color;

/// Options for diff output.
#[derive(Debug, Clone, Default)]
pub struct DiffOptions {
    /// Number of context lines before/after changes.
    pub context: usize,
    /// Show whitespace characters visually.
    pub show_whitespace: bool,
    /// Show file path header (git-style).
    pub path: Option<String>,
    /// Show byte length info (useful for debugging).
    pub show_lengths: bool,
}

impl DiffOptions {
    /// Create default options with 2 lines of context.
    pub fn new() -> Self {
        Self {
            context: 2,
            show_whitespace: false,
            path: None,
            show_lengths: false,
        }
    }

    /// Set context lines.
    pub fn with_context(mut self, context: usize) -> Self {
        self.context = context;
        self
    }

    /// Enable whitespace visualization.
    pub fn with_whitespace(mut self) -> Self {
        self.show_whitespace = true;
        self
    }

    /// Set file path for header.
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Show byte lengths (useful for debugging).
    pub fn with_lengths(mut self) -> Self {
        self.show_lengths = true;
        self
    }
}

/// Print a unified diff between two strings.
pub fn print_diff(expected: &str, actual: &str, options: &DiffOptions) {
    // byte length info for debugging
    if options.show_lengths {
        eprintln!(
            "  {} bytes: {}, {} bytes: {}",
            Color::Red.apply("expected"),
            expected.len(),
            Color::Green.apply("actual"),
            actual.len()
        );
        eprintln!();
    }

    // git-style header
    if let Some(path) = &options.path {
        eprintln!("{}", Color::White.apply(&format!("--- a/{path}")));
        eprintln!("{}", Color::White.apply(&format!("+++ b/{path}")));
    }

    let expected_lines: Vec<&str> = expected.lines().collect();
    let actual_lines: Vec<&str> = actual.lines().collect();

    // LCS-based diff
    let mut i = 0;
    let mut j = 0;
    let mut context_buffer: Vec<(usize, &str)> = Vec::new();
    let mut pending_removals: Vec<(usize, &str)> = Vec::new();
    let mut pending_additions: Vec<(usize, &str)> = Vec::new();
    let mut in_hunk = false;

    while i < expected_lines.len() || j < actual_lines.len() {
        // matching lines
        if i < expected_lines.len()
            && j < actual_lines.len()
            && expected_lines[i] == actual_lines[j]
        {
            // flush any pending changes
            if !pending_removals.is_empty() || !pending_additions.is_empty() {
                flush_changes(
                    &mut context_buffer,
                    &mut pending_removals,
                    &mut pending_additions,
                    options,
                    &mut in_hunk,
                );
            }

            context_buffer.push((i + 1, expected_lines[i]));
            if context_buffer.len() > options.context {
                // print trailing context from previous hunk
                if in_hunk && context_buffer.len() == options.context + 1 {
                    let (line_no, line) = context_buffer.remove(0);
                    print_context_line(line_no, line, options);
                } else {
                    context_buffer.remove(0);
                }
            }

            // end hunk after enough context
            if in_hunk && context_buffer.len() >= options.context {
                in_hunk = false;
            }

            i += 1;
            j += 1;
            continue;
        }

        // find next matching line using lookahead
        let mut expected_skip = 0;
        let mut actual_skip = 0;
        'outer: for look_ahead in 1..=20 {
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

        // no match found, consume rest
        if expected_skip == 0 && actual_skip == 0 {
            expected_skip = expected_lines.len().saturating_sub(i);
            actual_skip = actual_lines.len().saturating_sub(j);
        }

        // collect removals and additions
        for k in 0..expected_skip {
            pending_removals.push((i + k + 1, expected_lines[i + k]));
        }
        for k in 0..actual_skip {
            pending_additions.push((j + k + 1, actual_lines[j + k]));
        }

        i += expected_skip;
        j += actual_skip;
    }

    // flush remaining changes
    if !pending_removals.is_empty() || !pending_additions.is_empty() {
        flush_changes(
            &mut context_buffer,
            &mut pending_removals,
            &mut pending_additions,
            options,
            &mut in_hunk,
        );
    }

    // trailing newline difference
    let expected_newline = expected.ends_with('\n');
    let actual_newline = actual.ends_with('\n');
    if expected_newline != actual_newline {
        eprintln!(
            "{}",
            Color::BrightYellow.apply(&format!(
                "\\ No newline at end of file (expected: {}, actual: {})",
                if expected_newline { "yes" } else { "no" },
                if actual_newline { "yes" } else { "no" }
            ))
        );
    }

    // whitespace-only difference note
    if expected.trim() == actual.trim() && expected != actual {
        eprintln!();
        eprintln!(
            "{}",
            Color::BrightYellow.apply("note: strings differ only in whitespace")
        );
    }
}

fn flush_changes(
    context_buffer: &mut Vec<(usize, &str)>,
    pending_removals: &mut Vec<(usize, &str)>,
    pending_additions: &mut Vec<(usize, &str)>,
    options: &DiffOptions,
    in_hunk: &mut bool,
) {
    // start new hunk if needed
    if !*in_hunk {
        *in_hunk = true;
        eprintln!();
    }

    // print context before
    for (line_no, line) in context_buffer.drain(..) {
        print_context_line(line_no, line, options);
    }

    // print removals
    for (line_no, line) in pending_removals.drain(..) {
        print_removal_line(line_no, line, options);
    }

    // print additions
    for (line_no, line) in pending_additions.drain(..) {
        print_addition_line(line_no, line, options);
    }
}

fn print_context_line(line_no: usize, line: &str, options: &DiffOptions) {
    let display = if options.show_whitespace {
        escape_whitespace(line)
    } else {
        line.to_string()
    };
    // dim gutter for context lines
    eprintln!("{} {display}", dim(&format!(" {line_no:>4}│")));
}

/// Apply dim ANSI styling.
fn dim(s: &str) -> String {
    format!("\x1b[2m{s}\x1b[0m")
}

fn print_removal_line(line_no: usize, line: &str, options: &DiffOptions) {
    let display = if options.show_whitespace {
        escape_whitespace(line)
    } else {
        line.to_string()
    };
    eprintln!(
        "{} {}",
        Color::BrightRed.apply(&format!("-{line_no:>4}│")),
        Color::Red.apply(&display)
    );
}

fn print_addition_line(line_no: usize, line: &str, options: &DiffOptions) {
    let display = if options.show_whitespace {
        escape_whitespace(line)
    } else {
        line.to_string()
    };
    eprintln!(
        "{} {}",
        Color::BrightGreen.apply(&format!("+{line_no:>4}│")),
        Color::Green.apply(&display)
    );
}

/// Escape whitespace characters for visual display.
pub fn escape_whitespace(s: &str) -> String {
    s.replace('\t', "→").replace('\r', "⏎").replace(' ', "·")
}