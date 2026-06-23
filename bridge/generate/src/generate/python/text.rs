pub(super) struct Text {
    /// Generated source.
    source: String,
}

impl Text {
    /// Create one empty text document.
    pub(super) fn new() -> Self {
        Self {
            source: String::new(),
        }
    }

    /// Create one generated text document.
    pub(super) fn generated() -> Self {
        let mut text = Self::new();
        text.line("# generated bridge target, do not edit");
        text.blank();

        text
    }

    /// Write raw generated source.
    pub(super) fn raw(&mut self, source: impl AsRef<str>) {
        self.source.push_str(source.as_ref());
    }

    /// Write one line.
    pub(super) fn line(&mut self, line: impl AsRef<str>) {
        self.source.push_str(line.as_ref());
        self.source.push('\n');
    }

    /// Write one blank line.
    pub(super) fn blank(&mut self) {
        self.source.push('\n');
    }

    /// Write one Python stub documentation line.
    pub(super) fn doc(&mut self, doc: &str, indent: &str) {
        if !doc.is_empty() {
            self.line(format!("{indent}\"\"\"{doc}\"\"\""));
        }
    }

    /// Return the generated source.
    pub(super) fn finish(self) -> String {
        self.source
    }
}
