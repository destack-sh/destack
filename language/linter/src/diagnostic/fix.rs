use destack_source::{Applicability, FileId, LabeledSpan, Span, Suggestion, SuggestionStyle};

/// A suggested fix for a lint.
#[derive(Debug, Clone)]
pub struct LintFix {
    /// Description of the fix.
    pub description: String,
    /// Text edits to apply.
    pub edits: Vec<TextEdit>,
    /// How safe the fix is to apply automatically.
    pub applicability: FixApplicability,
}

impl LintFix {
    /// Create a new lint fix.
    pub fn new(description: impl Into<String>, applicability: FixApplicability) -> Self {
        Self {
            description: description.into(),
            edits: Vec::new(),
            applicability,
        }
    }

    /// Create a safe fix (can be applied with `--fix`).
    pub fn safe(description: impl Into<String>) -> Self {
        Self::new(description, FixApplicability::Safe)
    }

    /// Create an unsafe fix (requires `--fix-unsafe`).
    pub fn r#unsafe(description: impl Into<String>) -> Self {
        Self::new(description, FixApplicability::Unsafe)
    }

    /// Create a suggestion (human review required).
    pub fn suggestion(description: impl Into<String>) -> Self {
        Self::new(description, FixApplicability::Suggestion)
    }

    /// Add a text edit.
    pub fn with_edit(mut self, edit: TextEdit) -> Self {
        self.edits.push(edit);
        self
    }

    /// Add a replacement edit.
    pub fn replace(mut self, span: Span, replacement: impl Into<String>) -> Self {
        self.edits.push(TextEdit::replace(span, replacement));
        self
    }

    /// Add a deletion edit.
    pub fn delete(mut self, span: Span) -> Self {
        self.edits.push(TextEdit::delete(span));
        self
    }

    /// Add an insertion edit at a position in a file.
    pub fn insert(mut self, file_id: FileId, position: u32, text: impl Into<String>) -> Self {
        self.edits.push(TextEdit::insert(file_id, position, text));
        self
    }

    /// Convert to a Suggestion.
    pub(crate) fn into_suggestion(self) -> Suggestion {
        // for now, we only support single-span fixes in the Suggestion format
        // multi-span fixes need more sophisticated handling
        let (spans, replacement) = if self.edits.len() == 1 {
            let edit = &self.edits[0];
            (
                vec![LabeledSpan {
                    span: edit.span,
                    label: self.description.clone(),
                }],
                Some(edit.replacement.clone()),
            )
        } else {
            // multiple edits: show spans but can't express replacement simply
            (
                self.edits
                    .iter()
                    .map(|e| LabeledSpan {
                        span: e.span,
                        label: String::new(),
                    })
                    .collect(),
                None,
            )
        };

        Suggestion {
            spans,
            replacement,
            message: self.description,
            style: SuggestionStyle::Normal,
            applicability: match self.applicability {
                FixApplicability::Safe => Applicability::Automatic,
                FixApplicability::Unsafe | FixApplicability::Suggestion => Applicability::Dangerous,
            },
        }
    }
}

/// A text edit for a fix.
#[derive(Debug, Clone)]
pub struct TextEdit {
    /// The span to replace.
    pub span: Span,
    /// The replacement text (empty = deletion).
    pub replacement: String,
}

impl TextEdit {
    /// Create a replacement edit.
    pub fn replace(span: Span, replacement: impl Into<String>) -> Self {
        Self {
            span,
            replacement: replacement.into(),
        }
    }

    /// Create a deletion edit.
    pub fn delete(span: Span) -> Self {
        Self {
            span,
            replacement: String::new(),
        }
    }

    /// Create an insertion edit at a position.
    pub fn insert(file_id: FileId, position: u32, text: impl Into<String>) -> Self {
        Self {
            span: Span::new(file_id, position, position),
            replacement: text.into(),
        }
    }
}

/// How safe a fix is to apply automatically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FixApplicability {
    /// Safe to apply automatically with `--fix`.
    /// The fix preserves the program's semantics.
    Safe,
    /// May change program semantics; requires `--fix-unsafe`.
    /// The fix might alter behavior in edge cases.
    Unsafe,
    /// Suggestion only; requires human review.
    /// The fix has placeholders or needs contextual judgment.
    Suggestion,
}
