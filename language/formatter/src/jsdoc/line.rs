/// Append-only line buffer backed by a single `String`.
///
/// Reduces per-line heap allocations during serialization: all content goes into
/// one contiguous buffer separated by `\n`. Call `into_string()` at the end to
/// get the final `\n`-separated string with zero allocation.
pub(super) struct LineBuffer {
    buf: String,
    /// Whether at least one line has been pushed.
    has_content: bool,
}

impl LineBuffer {
    /// Create an empty line buffer.
    pub(super) fn new() -> Self {
        Self {
            buf: String::new(),
            has_content: false,
        }
    }

    /// Push a line (or multiple lines if the string contains embedded `\n`).
    /// Each `\n` in the input creates a new line in the buffer.
    pub(super) fn push(&mut self, line: impl AsRef<str>) {
        let s = line.as_ref();
        if self.has_content {
            self.buf.push('\n');
        }
        self.buf.push_str(s);
        self.has_content = true;
    }

    /// Push an empty line.
    pub(super) fn push_empty(&mut self) {
        self.push("");
    }

    /// Start a new line and return the buffer for direct writes.
    /// Callers **must** write content before calling `last_is_empty()` or
    /// `push_empty()`, since the separator `\n` is already appended.
    pub(super) fn begin_line(&mut self) -> &mut String {
        if self.has_content {
            self.buf.push('\n');
        }
        self.has_content = true;
        &mut self.buf
    }

    /// Return whether the last pushed line was empty.
    pub(super) fn last_is_empty(&self) -> bool {
        self.has_content && (self.buf.is_empty() || self.buf.ends_with('\n'))
    }

    /// Return whether no lines have been pushed.
    pub(super) fn is_empty(&self) -> bool {
        !self.has_content
    }

    /// Return whether the last non-empty line ends a block-level element.
    pub(super) fn last_line_is_block_end(&self) -> bool {
        let last = self.last_non_empty_line();
        let trimmed = last.trim_start();

        // unordered lists and fenced code
        if trimmed.starts_with("- ")
            || trimmed.starts_with("+ ")
            || trimmed.starts_with("* ")
            || trimmed.starts_with("```")
        {
            return true;
        }

        // ordered lists
        if let Some(rest) = trimmed.strip_prefix(|c: char| c.is_ascii_digit()) {
            let rest = rest.trim_start_matches(|c: char| c.is_ascii_digit());
            if rest.starts_with(". ") {
                return true;
            }
        }
        // indented code blocks
        if last.starts_with("    ") {
            return true;
        }
        false
    }

    /// Return whether the last non-empty line is a code fence.
    pub(super) fn last_line_is_code_fence(&self) -> bool {
        self.last_non_empty_line().trim_start().starts_with("```")
    }

    /// Return the last non-empty line.
    fn last_non_empty_line(&self) -> &str {
        self.buf.rsplit('\n').find(|l| !l.is_empty()).unwrap_or("")
    }

    /// Return the line buffer as a string.
    pub(super) fn into_string(self) -> String {
        self.buf
    }
}
