use crate::{BatchEdit, Edit, FileEdit, FileId, Span};

/// Builder for constructing multiple edits for a single file.
///
/// Provides a fluent API for building up edits.
#[derive(Debug, Clone)]
pub struct EditBuilder {
    file: FileId,
    edits: Vec<Edit>,
}

impl EditBuilder {
    /// Create a new builder for the given file.
    pub fn new(file: FileId) -> Self {
        Self {
            file,
            edits: Vec::new(),
        }
    }

    /// Add a replacement edit.
    pub fn replace(mut self, span: Span, text: impl Into<String>) -> Self {
        self.edits.push(Edit::replace(span, text));
        self
    }

    /// Add a deletion edit.
    pub fn delete(mut self, span: Span) -> Self {
        self.edits.push(Edit::delete(span));
        self
    }

    /// Add an insertion edit.
    pub fn insert(mut self, position: u32, text: impl Into<String>) -> Self {
        self.edits.push(Edit::insert(self.file, position, text));
        self
    }

    /// Build into a FileEdit.
    pub fn build(self) -> FileEdit {
        FileEdit::with_edits(self.file, self.edits)
    }

    /// Build into a BatchEdit (single file).
    pub fn into_batch(self) -> BatchEdit {
        BatchEdit::single(self.build())
    }
}

/// Extension trait for BatchEdit with convenience methods.
pub trait BatchEditExt {
    /// Create a batch edit with a single replacement.
    fn single_replace(span: Span, text: impl Into<String>) -> BatchEdit;

    /// Create a batch edit with a single deletion.
    fn single_delete(span: Span) -> BatchEdit;

    /// Create a batch edit with a single insertion.
    fn single_insert(file: FileId, position: u32, text: impl Into<String>) -> BatchEdit;
}

impl BatchEditExt for BatchEdit {
    fn single_replace(span: Span, text: impl Into<String>) -> BatchEdit {
        BatchEdit::single(FileEdit::with_edits(
            span.file,
            vec![Edit::replace(span, text)],
        ))
    }

    fn single_delete(span: Span) -> BatchEdit {
        BatchEdit::single(FileEdit::with_edits(span.file, vec![Edit::delete(span)]))
    }

    fn single_insert(file: FileId, position: u32, text: impl Into<String>) -> BatchEdit {
        BatchEdit::single(FileEdit::with_edits(
            file,
            vec![Edit::insert(file, position, text)],
        ))
    }
}
