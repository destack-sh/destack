/// Append-only documentation lines stored in one string.
pub(super) struct LineBuffer {
    /// The lines separated by newlines.
    text: String,
    /// Whether at least one line exists.
    has_lines: bool,
}

impl LineBuffer {
    /// Create an empty line buffer.
    pub(super) fn new() -> Self {
        Self {
            text: String::new(),
            has_lines: false,
        }
    }

    /// Append one or more lines.
    pub(super) fn push(&mut self, line: impl AsRef<str>) {
        let line = line.as_ref();
        if self.has_lines {
            self.text.push('\n');
        }

        self.text.push_str(line);
        self.has_lines = true;
    }

    /// Push an empty line.
    pub(super) fn push_empty(&mut self) {
        self.push("");
    }

    /// Start a line and return its string storage.
    pub(super) fn begin_line(&mut self) -> &mut String {
        if self.has_lines {
            self.text.push('\n');
        }

        self.has_lines = true;

        &mut self.text
    }

    /// Return whether the last pushed line was empty.
    pub(super) fn last_is_empty(&self) -> bool {
        self.has_lines && (self.text.is_empty() || self.text.ends_with('\n'))
    }

    /// Return whether no lines have been pushed.
    pub(super) fn is_empty(&self) -> bool {
        !self.has_lines
    }

    /// Return the line buffer as a string.
    pub(super) fn into_string(self) -> String {
        self.text
    }
}
