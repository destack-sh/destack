pub(super) struct Text {
    /// Generated source.
    source: String,
}

impl Text {
    /// Create one empty text writer.
    pub(super) fn new() -> Self {
        Self {
            source: String::new(),
        }
    }

    /// Write one source line.
    pub(super) fn line(&mut self, line: impl AsRef<str>) {
        self.source.push_str(line.as_ref());
        self.source.push('\n');
    }

    /// Write one blank line.
    pub(super) fn blank(&mut self) {
        self.source.push('\n');
    }

    /// Return the generated text.
    pub(super) fn finish(self) -> String {
        self.source
    }
}
