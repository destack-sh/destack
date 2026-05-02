use destack_artifact::{DiagnosticAnchor, DiagnosticContext, DiagnosticError};
use destack_source::{Applicability, BatchEdit, DiagnosticSuggestion, Edit, FileId, Span};

/// A suggested fix for a lint.
#[derive(Debug, Clone)]
pub struct LintFix {
    /// Description of the fix.
    pub description: String,
    /// Edits to apply.
    pub edits: Vec<Edit>,
    /// How safe the fix is to apply automatically.
    pub applicability: Fixability,
}

impl LintFix {
    /// Create a new lint fix.
    pub fn new(description: impl Into<String>, applicability: Fixability) -> Self {
        Self {
            description: description.into(),
            edits: Vec::new(),
            applicability,
        }
    }

    /// Create a safe fix (can be applied with `--fix`).
    pub fn safe(description: impl Into<String>) -> Self {
        Self::new(description, Fixability::Safe)
    }

    /// Create an unsafe fix (requires `--fix-unsafe`).
    pub fn r#unsafe(description: impl Into<String>) -> Self {
        Self::new(description, Fixability::Unsafe)
    }

    /// Create a suggestion (human review required).
    pub fn suggestion(description: impl Into<String>) -> Self {
        Self::new(description, Fixability::Suggestion)
    }

    /// Add an edit.
    pub fn with_edit(mut self, edit: Edit) -> Self {
        self.edits.push(edit);
        self
    }

    /// Add multiple edits.
    pub fn with_edits(mut self, edits: impl IntoIterator<Item = Edit>) -> Self {
        self.edits.extend(edits);
        self
    }

    /// Add a replacement edit.
    pub fn replace(mut self, span: Span, new_text: impl Into<String>) -> Self {
        self.edits.push(Edit::replace(span, new_text));
        self
    }

    /// Add a deletion edit.
    pub fn delete(mut self, span: Span) -> Self {
        self.edits.push(Edit::delete(span));
        self
    }

    /// Add an insertion edit at a position in a file.
    pub fn insert(mut self, file: FileId, position: u32, text: impl Into<String>) -> Self {
        self.edits.push(Edit::insert(file, position, text));
        self
    }

    /// Convert to a final diagnostic suggestion.
    pub(crate) fn into_suggestion(
        &self,
        context: &dyn DiagnosticContext,
    ) -> Result<DiagnosticSuggestion, DiagnosticError> {
        let edits = self
            .edits
            .iter()
            .cloned()
            .fold(BatchEdit::new(), |mut batch, edit| {
                batch.add(edit);
                batch
            });
        let labels = self
            .edits
            .iter()
            .enumerate()
            .map(|(index, edit)| {
                if index == 0 {
                    context.label(
                        &DiagnosticAnchor::Span(edit.span),
                        Some(self.description.clone()),
                    )
                } else {
                    context.label(&DiagnosticAnchor::Span(edit.span), None)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        let applicability = match self.applicability {
            Fixability::Safe => Applicability::Automatic,
            Fixability::Unsafe => Applicability::Unsafe,
            Fixability::Suggestion => Applicability::Dangerous,
        };
        let mut suggestion =
            DiagnosticSuggestion::new(self.description.clone(), edits, applicability);
        suggestion.labels = labels;

        Ok(suggestion)
    }
}

/// How safe a fix is to apply automatically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Fixability {
    /// Safe to apply automatically with `--fix`.
    /// The fix preserves the repository's semantics.
    Safe,
    /// May change repository semantics; requires `--fix-unsafe`.
    /// The fix might alter behavior in edge cases.
    Unsafe,
    /// Suggestion only; requires human review.
    /// The fix has placeholders or needs contextual judgment.
    Suggestion,
}
