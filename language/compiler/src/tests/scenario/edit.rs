use std::path::PathBuf;

/// One file edit applied to a scenario workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CompilerEdit {
    /// Replace one file with new UTF-8 content.
    ReplaceFile { path: PathBuf, content: String },
}

impl CompilerEdit {
    /// Replace one file with new UTF-8 content.
    pub(crate) fn replace_file(path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        Self::ReplaceFile {
            path: path.into(),
            content: content.into(),
        }
    }
}

/// An ordered edit script for one scenario workspace.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CompilerEditScript {
    /// The ordered edits.
    edits: Vec<CompilerEdit>,
}

impl CompilerEditScript {
    /// Create an empty edit script.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Append one edit to the script.
    pub(crate) fn edit(mut self, edit: CompilerEdit) -> Self {
        self.edits.push(edit);
        self
    }

    /// Append one file replacement.
    pub(crate) fn replace_file(self, path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        self.edit(CompilerEdit::replace_file(path, content))
    }

    /// Iterate the contained edits.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &CompilerEdit> {
        self.edits.iter()
    }
}
