use crate::{BatchEdit, Edit, FileEdit, FileId, Span};

/// Builder for constructing multiple edits for a single file.
///
/// Provides a fluent API for building up edits. Can optionally hold a reference
/// to source text for operations like `wrap` and `replace_with` that need to
/// read existing content.
///
/// # Example
///
/// ```ignore
/// let edit = EditBuilder::from_file(file_id, source)
///     .insert_before(span, "await ")
///     .wrap(span, "(", ")")
///     .replace_with(other_span, |text| format!("Object.is({text}, -0)"))
///     .into_edits();
/// ```
#[derive(Debug, Clone)]
pub struct EditBuilder<'a> {
    file: FileId,
    source: Option<&'a str>,
    edits: Vec<Edit>,
}

impl<'a> EditBuilder<'a> {
    /// Create a new builder with source text for text-aware operations.
    pub fn from_file(file: FileId, source: &'a str) -> Self {
        Self {
            file,
            source: Some(source),
            edits: Vec::new(),
        }
    }

    /// Get the text at a span. Panics if source was not provided.
    pub fn get_text(&self, span: Span) -> &str {
        let source = self
            .source
            .expect("get_text requires EditBuilder::with_source");
        &source[span.start as usize..span.end as usize]
    }

    /// Get the text at a span, returning None if source was not provided.
    pub fn try_get_text(&self, span: Span) -> Option<&str> {
        self.source
            .map(|s| &s[span.start as usize..span.end as usize])
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

    /// Add an insertion edit at an absolute position.
    pub fn insert(mut self, position: u32, text: impl Into<String>) -> Self {
        self.edits.push(Edit::insert(self.file, position, text));
        self
    }

    /// Insert text before a span.
    pub fn insert_before(mut self, span: Span, text: impl Into<String>) -> Self {
        self.edits.push(Edit::insert(self.file, span.start, text));
        self
    }

    /// Insert text after a span.
    pub fn insert_after(mut self, span: Span, text: impl Into<String>) -> Self {
        self.edits.push(Edit::insert(self.file, span.end, text));
        self
    }

    /// Wrap content at span with prefix and suffix.
    ///
    /// Requires source text (use `with_source`).
    ///
    /// # Example
    /// ```ignore
    /// // expr -> (expr)
    /// builder.wrap(expr_span, "(", ")")
    ///
    /// // call() -> await call()
    /// builder.wrap(call_span, "await ", "")
    /// ```
    pub fn wrap(self, span: Span, prefix: &str, suffix: &str) -> Self {
        let content = self.get_text(span);
        let new_text = format!("{prefix}{content}{suffix}");
        self.replace(span, new_text)
    }

    /// Replace span using a function that receives the current text.
    ///
    /// Requires source text (use `with_source`).
    ///
    /// # Example
    /// ```ignore
    /// // x === -0 -> Object.is(x, -0)
    /// builder.replace_with(expr_span, |text| format!("Object.is({text}, -0)"))
    /// ```
    pub fn replace_with(self, span: Span, f: impl FnOnce(&str) -> String) -> Self {
        let content = self.get_text(span);
        let new_text = f(content);
        self.replace(span, new_text)
    }

    // -------------------------------------------------------------------------
    // Build methods
    // -------------------------------------------------------------------------

    /// Get the accumulated edits.
    pub fn edits(&self) -> &[Edit] {
        &self.edits
    }

    /// Take the accumulated edits.
    pub fn into_edits(self) -> Vec<Edit> {
        self.edits
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
