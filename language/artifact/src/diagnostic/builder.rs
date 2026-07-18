use destack_source::{
    Diagnostic, DiagnosticHelp, DiagnosticNote, DiagnosticSuggestion, DiagnosticTag,
};

use crate::{DiagnosticAnchor, DiagnosticContext, DiagnosticError, ToDiagnostic};

/// One secondary diagnostic label.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SecondaryLabel {
    /// The source anchor.
    anchor: DiagnosticAnchor,
    /// The label message.
    message: String,
}

impl SecondaryLabel {
    /// Create one secondary source label.
    fn new(anchor: impl Into<DiagnosticAnchor>, message: impl Into<String>) -> Self {
        Self {
            anchor: anchor.into(),
            message: message.into(),
        }
    }
}

/// Additional diagnostic fields attached to one provider diagnostic.
#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticBuilder<T> {
    /// The provider diagnostic value.
    diagnostic: T,
    /// The optional primary label message.
    primary: Option<String>,
    /// Secondary source labels to add.
    labels: Vec<SecondaryLabel>,
    /// Notes to add.
    notes: Vec<DiagnosticNote>,
    /// Help messages to add.
    helps: Vec<DiagnosticHelp>,
    /// Source edit suggestions to add.
    suggestions: Vec<DiagnosticSuggestion>,
    /// Diagnostic tags to add.
    tags: Vec<DiagnosticTag>,
}

impl<T> DiagnosticBuilder<T> {
    /// Create a diagnostic builder.
    pub fn new(diagnostic: T) -> Self {
        Self {
            diagnostic,
            primary: None,
            labels: Vec::new(),
            notes: Vec::new(),
            helps: Vec::new(),
            suggestions: Vec::new(),
            tags: Vec::new(),
        }
    }

    /// Return the wrapped provider diagnostic.
    pub fn diagnostic(&self) -> &T {
        &self.diagnostic
    }

    /// Return the wrapped provider diagnostic mutably.
    pub fn diagnostic_mut(&mut self) -> &mut T {
        &mut self.diagnostic
    }

    /// Set the primary label message.
    pub fn primary(mut self, message: impl Into<String>) -> Self {
        self.primary = Some(message.into());

        self
    }

    /// Add one secondary source label.
    pub fn label(
        mut self,
        anchor: impl Into<DiagnosticAnchor>,
        message: impl Into<String>,
    ) -> Self {
        self.labels.push(SecondaryLabel::new(anchor, message));
        self
    }

    /// Add one note.
    pub fn note(mut self, note: impl Into<DiagnosticNote>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Add one help message.
    pub fn help(mut self, help: impl Into<DiagnosticHelp>) -> Self {
        self.helps.push(help.into());
        self
    }

    /// Add one source edit suggestion.
    pub fn suggestion(mut self, suggestion: DiagnosticSuggestion) -> Self {
        self.suggestions.push(suggestion);
        self
    }

    /// Add one diagnostic tag.
    pub fn tag(mut self, tag: DiagnosticTag) -> Self {
        self.tags.push(tag);

        self
    }

    /// Return the wrapped provider diagnostic.
    pub fn into_inner(self) -> T {
        self.diagnostic
    }
}

impl<T> From<T> for DiagnosticBuilder<T> {
    /// Create a diagnostic builder from one provider diagnostic.
    fn from(diagnostic: T) -> Self {
        Self::new(diagnostic)
    }
}

impl<T> ToDiagnostic for DiagnosticBuilder<T>
where
    T: ToDiagnostic,
{
    /// Convert the provider diagnostic and its attached fields into a source diagnostic.
    fn to_diagnostic(
        &self,
        context: &dyn DiagnosticContext,
    ) -> Result<Diagnostic, DiagnosticError> {
        // base diagnostic
        let mut diagnostic = self.diagnostic.to_diagnostic(context)?;

        // set the primary label
        if let Some(message) = &self.primary {
            diagnostic.primary.message = Some(message.clone());
        }

        // secondary labels
        for label in &self.labels {
            let label = context.label(&label.anchor, Some(label.message.clone()))?;
            diagnostic = diagnostic.label(label);
        }

        // notes
        for note in &self.notes {
            diagnostic = diagnostic.note(note.clone());
        }

        // help messages
        for help in &self.helps {
            diagnostic = diagnostic.help(help.clone());
        }

        // suggestions
        for suggestion in &self.suggestions {
            diagnostic = diagnostic.suggestion(suggestion.clone());
        }

        // tags
        for tag in &self.tags {
            diagnostic = diagnostic.tag(*tag);
        }

        Ok(diagnostic)
    }
}
