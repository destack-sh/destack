use destack_core::Color;

/// Options for diff output.
#[derive(Debug, Clone)]
pub struct DiffOptions {
    /// Number of context lines before/after changes.
    pub context: usize,
    /// Show whitespace characters visually.
    pub show_whitespace: bool,
    /// Show file path header (git-style).
    pub path: Option<String>,
    /// Show byte length info (useful for debugging).
    pub show_lengths: bool,
    /// Whether to emit ANSI color escape sequences.
    pub use_color: bool,
}

impl Default for DiffOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl DiffOptions {
    /// Create default options with 2 lines of context.
    pub fn new() -> Self {
        Self {
            context: 2,
            show_whitespace: false,
            path: None,
            show_lengths: false,
            use_color: true,
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

    /// Set whether ANSI color escape sequences are emitted.
    pub fn with_color(mut self, use_color: bool) -> Self {
        self.use_color = use_color;
        self
    }
}

/// Print a unified diff between two strings.
pub fn print_diff(expected: &str, actual: &str, options: &DiffOptions) {
    eprint!("{}", format_diff(expected, actual, options));
}

/// Format a unified diff between two strings.
pub fn format_diff(expected: &str, actual: &str, options: &DiffOptions) -> String {
    let mut output = String::new();

    // byte length info for debugging
    if options.show_lengths {
        let length_line = format!(
            "  {} bytes: {}, {} bytes: {}",
            color_text(options, Color::Red, "expected"),
            expected.len(),
            color_text(options, Color::Green, "actual"),
            actual.len(),
        );

        output.push_str(&length_line);
        output.push('\n');
        output.push('\n');
    }

    // git style header
    if let Some(path) = &options.path {
        output.push_str(&color_text(options, Color::White, &format!("--- a/{path}")));
        output.push('\n');
        output.push_str(&color_text(options, Color::White, &format!("+++ b/{path}")));
        output.push('\n');
    }

    // collect lines
    let expected_lines: Vec<&str> = expected.lines().collect();
    let actual_lines: Vec<&str> = actual.lines().collect();

    // lcs diff
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
                    &mut output,
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
                    print_context_line(&mut output, line_no, line, options);
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

        // consume the rest when no match is found
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
            &mut output,
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
        output.push_str(&color_text(
            options,
            Color::BrightYellow,
            &format!(
                "\\ No newline at end of file (expected: {}, actual: {})",
                if expected_newline { "yes" } else { "no" },
                if actual_newline { "yes" } else { "no" }
            ),
        ));
        output.push('\n');
    }

    // whitespace difference note
    if expected.trim() == actual.trim() && expected != actual {
        output.push('\n');
        output.push_str(&color_text(
            options,
            Color::BrightYellow,
            "note: strings differ only in whitespace",
        ));
        output.push('\n');
    }

    output
}

/// Flush buffered changes into the diff output.
fn flush_changes(
    output: &mut String,
    context_buffer: &mut Vec<(usize, &str)>,
    pending_removals: &mut Vec<(usize, &str)>,
    pending_additions: &mut Vec<(usize, &str)>,
    options: &DiffOptions,
    in_hunk: &mut bool,
) {
    // start new hunk if needed
    if !*in_hunk {
        *in_hunk = true;
        output.push('\n');
    }

    // print context before
    for (line_no, line) in context_buffer.drain(..) {
        print_context_line(output, line_no, line, options);
    }

    // print removals
    for (line_no, line) in pending_removals.drain(..) {
        print_removal_line(output, line_no, line, options);
    }

    // print additions
    for (line_no, line) in pending_additions.drain(..) {
        print_addition_line(output, line_no, line, options);
    }
}

/// Write one context line into the diff output.
fn print_context_line(output: &mut String, line_no: usize, line: &str, options: &DiffOptions) {
    let display = if options.show_whitespace {
        escape_whitespace(line)
    } else {
        line.to_string()
    };

    // dim gutter for context lines
    output.push_str(&format!(
        "{} {display}",
        dim(options, &format!(" {line_no:>4}│"))
    ));
    output.push('\n');
}

/// Apply dim ANSI styling.
fn dim(options: &DiffOptions, s: &str) -> String {
    if options.use_color {
        format!("\x1b[2m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

/// Write one removal line into the diff output.
fn print_removal_line(output: &mut String, line_no: usize, line: &str, options: &DiffOptions) {
    let display = if options.show_whitespace {
        escape_whitespace(line)
    } else {
        line.to_string()
    };

    output.push_str(&format!(
        "{} {}",
        color_text(options, Color::BrightRed, &format!("-{line_no:>4}│")),
        color_text(options, Color::Red, &display)
    ));
    output.push('\n');
}

/// Write one addition line into the diff output.
fn print_addition_line(output: &mut String, line_no: usize, line: &str, options: &DiffOptions) {
    let display = if options.show_whitespace {
        escape_whitespace(line)
    } else {
        line.to_string()
    };

    output.push_str(&format!(
        "{} {}",
        color_text(options, Color::BrightGreen, &format!("+{line_no:>4}│")),
        color_text(options, Color::Green, &display)
    ));
    output.push('\n');
}

/// Escape whitespace characters for visual display.
pub fn escape_whitespace(s: &str) -> String {
    s.replace('\t', "→").replace('\r', "⏎").replace(' ', "·")
}

/// Apply color when output colors are enabled.
fn color_text(options: &DiffOptions, color: Color, text: &str) -> String {
    if options.use_color {
        color.apply(text)
    } else {
        text.to_string()
    }
}
