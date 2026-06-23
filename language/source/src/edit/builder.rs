use crate::{FileId, FilePatch, Patch, PatchSet, Span};

/// Builder for constructing multiple patches for a single file.
///
/// Provides a fluent API for building patches from optional source text.
///
/// # Example
///
/// ```ignore
/// let patch = PatchBuilder::from_file(file_id, source)
///     .insert_before(span, "await ")
///     .wrap(span, "(", ")")
///     .replace_with(other_span, |text| format!("Object.is({text}, -0)"))
///     .into_patches();
/// ```
#[derive(Debug, Clone)]
pub struct PatchBuilder<'a> {
    file: FileId,
    source: Option<&'a str>,
    patches: Vec<Patch>,
}

impl<'a> PatchBuilder<'a> {
    /// Create a new builder with source text for text-aware operations.
    pub fn from_file(file: FileId, source: &'a str) -> Self {
        Self {
            file,
            source: Some(source),
            patches: Vec::new(),
        }
    }

    /// Get the text at a span.
    pub fn get_text(&self, span: Span) -> &str {
        let source = self
            .source
            .expect("get_text requires PatchBuilder source text");
        &source[span.start as usize..span.end as usize]
    }

    /// Get the text at a span, returning None if source was not provided.
    pub fn try_get_text(&self, span: Span) -> Option<&str> {
        self.source
            .map(|s| &s[span.start as usize..span.end as usize])
    }

    /// Add a replacement patch.
    pub fn replace(mut self, span: Span, text: impl Into<String>) -> Self {
        self.patches.push(Patch::replace(span, text));
        self
    }

    /// Add a deletion patch.
    pub fn delete(mut self, span: Span) -> Self {
        self.patches.push(Patch::delete(span));
        self
    }

    /// Add an insertion patch at an absolute position.
    pub fn insert(mut self, position: u32, text: impl Into<String>) -> Self {
        self.patches.push(Patch::insert(self.file, position, text));
        self
    }

    /// Insert text before a span.
    pub fn insert_before(mut self, span: Span, text: impl Into<String>) -> Self {
        self.patches
            .push(Patch::insert(self.file, span.start, text));
        self
    }

    /// Insert text after a span.
    pub fn insert_after(mut self, span: Span, text: impl Into<String>) -> Self {
        self.patches.push(Patch::insert(self.file, span.end, text));
        self
    }

    /// Wrap content at span with prefix and suffix.
    ///
    /// Requires source text.
    ///
    /// # Example
    /// ```ignore
    /// // expr -> (expr)
    /// builder.wrap(expression_span, "(", ")")
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
    /// Requires source text.
    ///
    /// # Example
    /// ```ignore
    /// // x === -0 -> Object.is(x, -0)
    /// builder.replace_with(expression_span, |text| format!("Object.is({text}, -0)"))
    /// ```
    pub fn replace_with(self, span: Span, f: impl FnOnce(&str) -> String) -> Self {
        let content = self.get_text(span);
        let new_text = f(content);
        self.replace(span, new_text)
    }

    // -------------------------------------------------------------------------
    // Build methods
    // -------------------------------------------------------------------------

    /// Get the accumulated patches.
    pub fn patches(&self) -> &[Patch] {
        &self.patches
    }

    /// Take the accumulated patches.
    pub fn into_patches(self) -> Vec<Patch> {
        self.patches
    }

    /// Build into a FilePatch.
    pub fn build(self) -> FilePatch {
        FilePatch::with_patches(self.file, self.patches)
    }

    /// Build into a single-file PatchSet.
    pub fn into_patch_set(self) -> PatchSet {
        PatchSet::single(self.build())
    }
}
