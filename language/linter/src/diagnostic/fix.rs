use destack_artifact::{DiagnosticAnchor, DiagnosticContext, DiagnosticError};
use destack_source::{Applicability, DiagnosticSuggestion, FileId, Patch, PatchSet, Span};

/// A suggested fix for a lint.
#[derive(Debug, Clone)]
pub struct LintFix {
    /// Description of the fix.
    pub description: String,
    /// Patches to apply.
    pub patches: Vec<Patch>,
    /// How safe the fix is to apply automatically.
    pub applicability: Fixability,
}

impl LintFix {
    /// Create a new lint fix.
    pub fn new(description: impl Into<String>, applicability: Fixability) -> Self {
        Self {
            description: description.into(),
            patches: Vec::new(),
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

    /// Add one patch.
    pub fn with_patch(mut self, patch: Patch) -> Self {
        self.patches.push(patch);
        self
    }

    /// Add multiple patches.
    pub fn with_patches(mut self, patches: impl IntoIterator<Item = Patch>) -> Self {
        self.patches.extend(patches);
        self
    }

    /// Add a replacement patch.
    pub fn replace(mut self, span: Span, new_text: impl Into<String>) -> Self {
        self.patches.push(Patch::replace(span, new_text));
        self
    }

    /// Add a deletion patch.
    pub fn delete(mut self, span: Span) -> Self {
        self.patches.push(Patch::delete(span));
        self
    }

    /// Add an insertion patch at a position in a file.
    pub fn insert(mut self, file: FileId, position: u32, text: impl Into<String>) -> Self {
        self.patches.push(Patch::insert(file, position, text));
        self
    }

    /// Convert to a final diagnostic suggestion.
    pub(crate) fn to_suggestion(
        &self,
        context: &dyn DiagnosticContext,
    ) -> Result<DiagnosticSuggestion, DiagnosticError> {
        let patches = self
            .patches
            .iter()
            .cloned()
            .fold(PatchSet::new(), |mut batch, patch| {
                batch.add(patch);
                batch
            });
        let labels = self
            .patches
            .iter()
            .enumerate()
            .map(|(index, patch)| {
                if index == 0 {
                    context.label(
                        &DiagnosticAnchor::Span(patch.span),
                        Some(self.description.clone()),
                    )
                } else {
                    context.label(&DiagnosticAnchor::Span(patch.span), None)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        let applicability = match self.applicability {
            Fixability::Safe => Applicability::Automatic,
            Fixability::Unsafe => Applicability::Unsafe,
            Fixability::Suggestion => Applicability::Dangerous,
        };
        let mut suggestion =
            DiagnosticSuggestion::new(self.description.clone(), patches, applicability);
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
